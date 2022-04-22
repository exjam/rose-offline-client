use std::collections::HashMap;
use thiserror::Error;

use crate::scripting::lua4::{Lua4Function, Lua4Instruction, Lua4Value};

#[derive(Debug, Clone, Error)]
pub enum Lua4VMError {
    #[error("Missing value in stack")]
    MissingStackValue,

    #[error("Global {0} not found")]
    GlobalNotFound(String),

    #[error("Expected value to be a Closure")]
    NotClosure,

    #[error("Unimplemented instruction {0:?}")]
    Unimplemented(Lua4Instruction),
}
pub trait Lua4VMRustClosures {
    fn call_rust_closure(
        &mut self,
        name: &str,
        parameters: Vec<Lua4Value>,
    ) -> Result<Vec<Lua4Value>, Lua4VMError>;
}

#[derive(Default)]
pub struct Lua4VM {
    pub globals: HashMap<String, Lua4Value>,
}

impl Lua4VM {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_global(&mut self, name: String, value: Lua4Value) {
        self.globals.insert(name, value);
    }

    pub fn get_global(&mut self, name: &str) -> Option<&Lua4Value> {
        self.globals.get(name)
    }

    pub fn call_lua_function<T: Lua4VMRustClosures>(
        &mut self,
        rust_closures: &mut T,
        function: &Lua4Function,
        parameters: &[Lua4Value],
    ) -> Result<Vec<Lua4Value>, anyhow::Error> {
        let mut stack = Vec::with_capacity(function.max_stack_size as usize);
        let local_stack_index = stack.len();
        for i in 0..function.num_parameters as usize {
            stack.push(parameters.get(i).cloned().unwrap_or(Lua4Value::Nil));
        }

        let mut pc = 0;
        loop {
            let instruction = function.instructions[pc];
            pc += 1;
            match instruction {
                Lua4Instruction::OP_END => break,
                Lua4Instruction::OP_RETURN(return_stack_index) => {
                    // Leave only results on stack
                    stack.drain(0..local_stack_index + return_stack_index as usize);
                    break;
                }
                Lua4Instruction::OP_CALL(parameter_stack_index, num_results) => {
                    let parameters =
                        stack.split_off(local_stack_index + parameter_stack_index as usize + 1);
                    let closure = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;

                    let mut results = if let Lua4Value::Closure(function, _upvalues) = closure {
                        let function = function.clone();
                        self.call_lua_function(rust_closures, &function, &parameters)?
                    } else if let Lua4Value::RustClosure(function_name) = closure {
                        log::debug!(target: "lua", "Call rust closure: {}", function_name);
                        rust_closures.call_rust_closure(&function_name, parameters)?
                    } else {
                        return Err(Lua4VMError::NotClosure.into());
                    };

                    results.reverse();
                    for _ in 0..num_results {
                        stack.push(results.pop().unwrap_or(Lua4Value::Nil));
                    }
                }
                // TODO: Lua4Instruction::OP_TAILCALL(u32, u32)
                Lua4Instruction::OP_PUSHNIL(count) => {
                    for _ in 0..count {
                        stack.push(Lua4Value::Nil);
                    }
                }
                Lua4Instruction::OP_POP(count) => {
                    for _ in 0..count {
                        stack.pop();
                    }
                }
                Lua4Instruction::OP_PUSHINT(value) => {
                    stack.push(Lua4Value::Number(value as f64));
                }
                Lua4Instruction::OP_PUSHSTRING(kstr) => {
                    stack.push(Lua4Value::String(
                        function.constant_strings[kstr as usize].clone(),
                    ));
                }
                Lua4Instruction::OP_PUSHNUM(knum) => {
                    stack.push(Lua4Value::Number(function.constant_numbers[knum as usize]));
                }
                Lua4Instruction::OP_PUSHNEGNUM(knum) => {
                    stack.push(Lua4Value::Number(-function.constant_numbers[knum as usize]));
                }
                // TODO: Lua4Instruction::OP_PUSHUPVALUE(u32)
                Lua4Instruction::OP_GETLOCAL(index) => {
                    let value = stack
                        .get(local_stack_index + index as usize)
                        .ok_or(Lua4VMError::MissingStackValue)?
                        .clone();
                    stack.push(value);
                }
                Lua4Instruction::OP_GETGLOBAL(kstr) => {
                    let name = &function.constant_strings[kstr as usize];
                    let value = self
                        .get_global(name)
                        .ok_or_else(|| Lua4VMError::GlobalNotFound(name.into()))?
                        .clone();
                    stack.push(value);
                }
                // TODO: Lua4Instruction::OP_GETTABLE
                // TODO: Lua4Instruction::OP_GETDOTTED(u32)
                // TODO: Lua4Instruction::OP_GETINDEXED(u32)
                // TODO: Lua4Instruction::OP_PUSHSELF(u32)
                // TODO: Lua4Instruction::OP_CREATETABLE(u32)
                Lua4Instruction::OP_SETLOCAL(index) => {
                    stack[local_stack_index + index as usize] =
                        stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                }
                Lua4Instruction::OP_SETGLOBAL(kstr) => {
                    self.set_global(
                        function.constant_strings[kstr as usize].clone(),
                        stack.pop().ok_or(Lua4VMError::MissingStackValue)?,
                    );
                }
                // TODO: Lua4Instruction::OP_SETTABLE(u32, u32)
                // TODO: Lua4Instruction::OP_SETLIST(u32, u32)
                // TODO: Lua4Instruction::OP_SETMAP(u32)
                // TODO: Lua4Instruction::OP_ADD
                // TODO: Lua4Instruction::OP_ADDI(i32)
                // TODO: Lua4Instruction::OP_SUB
                // TODO: Lua4Instruction::OP_MULT
                // TODO: Lua4Instruction::OP_DIV
                // TODO: Lua4Instruction::OP_POW
                // TODO: Lua4Instruction::OP_CONCAT(u32)
                // TODO: Lua4Instruction::OP_MINUS
                // TODO: Lua4Instruction::OP_NOT
                Lua4Instruction::OP_JMPNE(target) => {
                    let rhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    let lhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;

                    if lhs != rhs {
                        pc = (pc as i32 + target) as usize;
                    }
                }
                Lua4Instruction::OP_JMPEQ(target) => {
                    let rhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    let lhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;

                    if lhs == rhs {
                        pc = (pc as i32 + target) as usize;
                    }
                }
                Lua4Instruction::OP_JMPLT(target) => {
                    let rhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    let lhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;

                    if lhs < rhs {
                        pc = (pc as i32 + target) as usize;
                    }
                }
                Lua4Instruction::OP_JMPLE(target) => {
                    let rhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    let lhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;

                    if lhs <= rhs {
                        pc = (pc as i32 + target) as usize;
                    }
                }
                Lua4Instruction::OP_JMPGT(target) => {
                    let rhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    let lhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;

                    if lhs > rhs {
                        pc = (pc as i32 + target) as usize;
                    }
                }
                Lua4Instruction::OP_JMPGE(target) => {
                    let rhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    let lhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;

                    if lhs >= rhs {
                        pc = (pc as i32 + target) as usize;
                    }
                }
                Lua4Instruction::OP_JMPT(target) => {
                    let value = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;

                    if !matches!(value, Lua4Value::Nil) {
                        pc = (pc as i32 + target) as usize;
                    }
                }
                Lua4Instruction::OP_JMPF(target) => {
                    let value = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;

                    if matches!(value, Lua4Value::Nil) {
                        pc = (pc as i32 + target) as usize;
                    }
                }
                Lua4Instruction::OP_JMPONT(target) => {
                    // If value on top of stack is Nil then pop it, else jump
                    let peek_value = stack.last().ok_or(Lua4VMError::MissingStackValue)?;

                    if matches!(peek_value, Lua4Value::Nil) {
                        stack.pop();
                    } else {
                        pc = (pc as i32 + target) as usize;
                    }
                }
                Lua4Instruction::OP_JMPONF(target) => {
                    // If value on top of stack is not Nil then pop it, else jump
                    let peek_value = stack.last().ok_or(Lua4VMError::MissingStackValue)?;

                    if !matches!(peek_value, Lua4Value::Nil) {
                        stack.pop();
                    } else {
                        pc = (pc as i32 + target) as usize;
                    }
                }
                Lua4Instruction::OP_JMP(target) => {
                    pc = (pc as i32 + target) as usize;
                }
                Lua4Instruction::OP_PUSHNILJMP => {
                    stack.push(Lua4Value::Nil);
                    pc = (pc as i32 + 1) as usize;
                }
                // TODO: Lua4Instruction::OP_FORPREP(i32)
                // TODO: Lua4Instruction::OP_FORLOOP(i32)
                // TODO: Lua4Instruction::OP_LFORPREP(i32)
                // TODO: Lua4Instruction::OP_LFORLOOP(i32)
                Lua4Instruction::OP_CLOSURE(kproto, b) => {
                    let upvalues = stack.split_off(stack.len() - b as usize);
                    stack.push(Lua4Value::Closure(
                        function.constant_functions[kproto as usize].clone(),
                        upvalues,
                    ));
                }
                _ => {
                    anyhow::bail!(Lua4VMError::Unimplemented(instruction))
                }
            }
        }

        Ok(stack)
    }

    pub fn call_global_closure<T: Lua4VMRustClosures>(
        &mut self,
        rust_closures: &mut T,
        name: &str,
        parameters: &[Lua4Value],
    ) -> Result<Vec<Lua4Value>, anyhow::Error> {
        let global_value = self
            .get_global(name)
            .ok_or_else(|| Lua4VMError::GlobalNotFound(name.into()))?;

        if let Lua4Value::Closure(function, _upvalues) = global_value {
            let function = function.clone();
            self.call_lua_function(rust_closures, &function, parameters)
        } else {
            Err(Lua4VMError::NotClosure.into())
        }
    }
}
