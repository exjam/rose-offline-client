#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Lua4Instruction {
    OP_END,
    OP_RETURN(u32),
    OP_CALL(u32, u32),
    OP_TAILCALL(u32, u32),
    OP_PUSHNIL(u32),
    OP_POP(u32),
    OP_PUSHINT(i32),
    OP_PUSHSTRING(u32),
    OP_PUSHNUM(u32),
    OP_PUSHNEGNUM(u32),
    OP_PUSHUPVALUE(u32),
    OP_GETLOCAL(u32),
    OP_GETGLOBAL(u32),
    OP_GETTABLE,
    OP_GETDOTTED(u32),
    OP_GETINDEXED(u32),
    OP_PUSHSELF(u32),
    OP_CREATETABLE(u32),
    OP_SETLOCAL(u32),
    OP_SETGLOBAL(u32),
    OP_SETTABLE(u32, u32),
    OP_SETLIST(u32, u32),
    OP_SETMAP(u32),
    OP_ADD,
    OP_ADDI(i32),
    OP_SUB,
    OP_MULT,
    OP_DIV,
    OP_POW,
    OP_CONCAT(u32),
    OP_MINUS,
    OP_NOT,
    OP_JMPNE(i32),
    OP_JMPEQ(i32),
    OP_JMPLT(i32),
    OP_JMPLE(i32),
    OP_JMPGT(i32),
    OP_JMPGE(i32),
    OP_JMPT(i32),
    OP_JMPF(i32),
    OP_JMPONT(i32),
    OP_JMPONF(i32),
    OP_JMP(i32),
    OP_PUSHNILJMP,
    OP_FORPREP(i32),
    OP_FORLOOP(i32),
    OP_LFORPREP(i32),
    OP_LFORLOOP(i32),
    OP_CLOSURE(u32, u32),
}

fn get_arg_u(instruction: u32) -> u32 {
    instruction >> 6
}

fn get_arg_s(instruction: u32) -> i32 {
    get_arg_u(instruction) as i32 - (((1 << 26) - 1) >> 1)
}

fn get_arg_a(instruction: u32) -> u32 {
    instruction >> (6 + 9)
}

fn get_arg_b(instruction: u32) -> u32 {
    (instruction >> 6) & 0b111111111
}

impl Lua4Instruction {
    pub fn from_u32(instruction: u32) -> Result<Self, anyhow::Error> {
        let opcode = instruction & 0b111111;
        let u = get_arg_u(instruction);
        let s = get_arg_s(instruction);
        let a = get_arg_a(instruction);
        let b = get_arg_b(instruction);

        match opcode {
            0 => Ok(Lua4Instruction::OP_END),
            1 => Ok(Lua4Instruction::OP_RETURN(u)),
            2 => Ok(Lua4Instruction::OP_CALL(a, b)),
            3 => Ok(Lua4Instruction::OP_TAILCALL(a, b)),
            4 => Ok(Lua4Instruction::OP_PUSHNIL(u)),
            5 => Ok(Lua4Instruction::OP_POP(u)),
            6 => Ok(Lua4Instruction::OP_PUSHINT(s)),
            7 => Ok(Lua4Instruction::OP_PUSHSTRING(u)),
            8 => Ok(Lua4Instruction::OP_PUSHNUM(u)),
            9 => Ok(Lua4Instruction::OP_PUSHNEGNUM(u)),
            10 => Ok(Lua4Instruction::OP_PUSHUPVALUE(u)),
            11 => Ok(Lua4Instruction::OP_GETLOCAL(u)),
            12 => Ok(Lua4Instruction::OP_GETGLOBAL(u)),
            13 => Ok(Lua4Instruction::OP_GETTABLE),
            14 => Ok(Lua4Instruction::OP_GETDOTTED(u)),
            15 => Ok(Lua4Instruction::OP_GETINDEXED(u)),
            16 => Ok(Lua4Instruction::OP_PUSHSELF(u)),
            17 => Ok(Lua4Instruction::OP_CREATETABLE(u)),
            18 => Ok(Lua4Instruction::OP_SETLOCAL(u)),
            19 => Ok(Lua4Instruction::OP_SETGLOBAL(u)),
            20 => Ok(Lua4Instruction::OP_SETTABLE(a, b)),
            21 => Ok(Lua4Instruction::OP_SETLIST(a, b)),
            22 => Ok(Lua4Instruction::OP_SETMAP(u)),
            23 => Ok(Lua4Instruction::OP_ADD),
            24 => Ok(Lua4Instruction::OP_ADDI(s)),
            25 => Ok(Lua4Instruction::OP_SUB),
            26 => Ok(Lua4Instruction::OP_MULT),
            27 => Ok(Lua4Instruction::OP_DIV),
            28 => Ok(Lua4Instruction::OP_POW),
            29 => Ok(Lua4Instruction::OP_CONCAT(u)),
            30 => Ok(Lua4Instruction::OP_MINUS),
            31 => Ok(Lua4Instruction::OP_NOT),
            32 => Ok(Lua4Instruction::OP_JMPNE(s)),
            33 => Ok(Lua4Instruction::OP_JMPEQ(s)),
            34 => Ok(Lua4Instruction::OP_JMPLT(s)),
            35 => Ok(Lua4Instruction::OP_JMPLE(s)),
            36 => Ok(Lua4Instruction::OP_JMPGT(s)),
            37 => Ok(Lua4Instruction::OP_JMPGE(s)),
            38 => Ok(Lua4Instruction::OP_JMPT(s)),
            39 => Ok(Lua4Instruction::OP_JMPF(s)),
            40 => Ok(Lua4Instruction::OP_JMPONT(s)),
            41 => Ok(Lua4Instruction::OP_JMPONF(s)),
            42 => Ok(Lua4Instruction::OP_JMP(s)),
            43 => Ok(Lua4Instruction::OP_PUSHNILJMP),
            44 => Ok(Lua4Instruction::OP_FORPREP(s)),
            45 => Ok(Lua4Instruction::OP_FORLOOP(s)),
            46 => Ok(Lua4Instruction::OP_LFORPREP(s)),
            47 => Ok(Lua4Instruction::OP_LFORLOOP(s)),
            48 => Ok(Lua4Instruction::OP_CLOSURE(a, b)),
            invalid => anyhow::bail!("Invalid instruction opcode {}", invalid),
        }
    }
}
