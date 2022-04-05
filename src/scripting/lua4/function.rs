use std::sync::Arc;

use rose_file_readers::RoseFileReader;

use crate::scripting::lua4::Lua4Instruction;

#[derive(Debug, Copy, Clone)]
enum LuaEndian {
    Little,
    Big,
}

#[derive(Debug, Clone)]
pub struct Lua4LocalVar {
    pub name: String,
    pub start_pc: u32,
    pub end_pc: u32,
}

#[derive(Debug, Clone)]
pub struct Lua4Function {
    pub source: String,
    pub line: u32,
    pub num_parameters: u32,
    pub is_var_arg: bool,
    pub max_stack_size: u32,
    pub local_vars: Vec<Lua4LocalVar>,
    pub line_infos: Vec<u32>,
    pub constant_strings: Vec<String>,
    pub constant_numbers: Vec<f64>,
    pub constant_functions: Vec<Arc<Lua4Function>>,
    pub instructions: Vec<Lua4Instruction>,
}

impl Lua4Function {
    pub fn from_bytes(bytes: &[u8]) -> Result<Arc<Lua4Function>, anyhow::Error> {
        let mut reader = RoseFileReader::from(bytes);
        let chunk_magic = reader.read_u8()?;
        if chunk_magic != 27 {
            anyhow::bail!("Expected lua chunk id, found {}", chunk_magic);
        }

        let endian = read_lua_header(&mut reader)?;
        read_lua_function(&mut reader, endian)
    }
}

fn read_lua_int(reader: &mut RoseFileReader, endian: LuaEndian) -> Result<u32, anyhow::Error> {
    let value = reader.read_u32()?;
    match endian {
        LuaEndian::Little => Ok(u32::from_le(value)),
        LuaEndian::Big => Ok(u32::from_be(value)),
    }
}

fn read_lua_number(reader: &mut RoseFileReader, endian: LuaEndian) -> Result<f64, anyhow::Error> {
    let value = reader.read_fixed_length_bytes(8)?;
    match endian {
        LuaEndian::Little => Ok(f64::from_le_bytes(value.try_into()?)),
        LuaEndian::Big => Ok(f64::from_be_bytes(value.try_into()?)),
    }
}

fn read_lua_string<'a>(
    reader: &mut RoseFileReader<'a>,
    endian: LuaEndian,
) -> Result<std::borrow::Cow<'a, str>, anyhow::Error> {
    let size = read_lua_int(reader, endian)? as usize;
    Ok(reader.read_fixed_length_string(size)?)
}

fn read_lua_header(reader: &mut RoseFileReader) -> Result<LuaEndian, anyhow::Error> {
    let magic = reader.read_fixed_length_string(3)?;
    if magic != "Lua" {
        anyhow::bail!("Invalid lua magic: {}", magic);
    }

    let version = reader.read_u8()?;
    if version != 0x40 {
        anyhow::bail!("Invalid lua version: {}", version);
    }

    let endian = match reader.read_u8()? {
        0 => LuaEndian::Big,
        1 => LuaEndian::Little,
        invalid => anyhow::bail!("Invalid lua endian: {}", invalid),
    };

    let sizeof_int = reader.read_u8()?;
    if sizeof_int != 4 {
        anyhow::bail!("Mismatch sizeof lua int, found: {}", sizeof_int);
    }

    let sizeof_size = reader.read_u8()?;
    if sizeof_size != 4 {
        anyhow::bail!("Mismatch sizeof lua size, found: {}", sizeof_size);
    }

    let sizeof_instruction = reader.read_u8()?;
    if sizeof_instruction != 4 {
        anyhow::bail!(
            "Mismatch sizeof lua Instruction, found: {}",
            sizeof_instruction
        );
    }

    let sizeof_instruction_bits = reader.read_u8()?;
    if sizeof_instruction_bits != 32 {
        anyhow::bail!(
            "Mismatch sizeof lua SIZE_INSTRUCTION, found: {}",
            sizeof_instruction_bits
        );
    }

    let sizeof_instruction_op_bits = reader.read_u8()?;
    if sizeof_instruction_op_bits != 6 {
        anyhow::bail!(
            "Mismatch sizeof lua SIZE_OP, found: {}",
            sizeof_instruction_op_bits
        );
    }

    let sizeof_instruction_b_bits = reader.read_u8()?;
    if sizeof_instruction_b_bits != 9 {
        anyhow::bail!(
            "Mismatch sizeof lua SIZE_B, found: {}",
            sizeof_instruction_b_bits
        );
    }

    let sizeof_number = reader.read_u8()?;
    if sizeof_number != 8 {
        anyhow::bail!("Mismatch sizeof lua number, found: {}", sizeof_number);
    }

    let lua_number = read_lua_number(reader, endian)?;
    if lua_number as i64 != (std::f64::consts::PI * 1E8) as i64 {
        anyhow::bail!("Failed number check, found value: {}", lua_number);
    }

    Ok(endian)
}

fn read_lua_function(
    reader: &mut RoseFileReader,
    endian: LuaEndian,
) -> Result<Arc<Lua4Function>, anyhow::Error> {
    let source = read_lua_string(reader, endian)?.to_string();
    let line = read_lua_int(reader, endian)?;
    let num_parameters = read_lua_int(reader, endian)?;
    let is_var_arg = reader.read_u8()? != 0;
    let max_stack_size = read_lua_int(reader, endian)?;

    let num_local_vars = read_lua_int(reader, endian)? as usize;
    let mut local_vars = Vec::with_capacity(num_local_vars);
    for _ in 0..num_local_vars {
        let name = read_lua_string(reader, endian)?.to_string();
        let start_pc = read_lua_int(reader, endian)?;
        let end_pc = read_lua_int(reader, endian)?;

        local_vars.push(Lua4LocalVar {
            name,
            start_pc,
            end_pc,
        });
    }

    let num_line_info = read_lua_int(reader, endian)? as usize;
    let mut line_infos = Vec::with_capacity(num_line_info);
    for _ in 0..num_line_info {
        line_infos.push(read_lua_int(reader, endian)?);
    }

    let num_constant_strings = read_lua_int(reader, endian)? as usize;
    let mut constant_strings = Vec::with_capacity(num_constant_strings);
    for _ in 0..num_constant_strings {
        constant_strings.push(read_lua_string(reader, endian)?.to_string());
    }

    let num_constant_numbers = read_lua_int(reader, endian)? as usize;
    let mut constant_numbers = Vec::with_capacity(num_constant_numbers);
    for _ in 0..num_constant_numbers {
        constant_numbers.push(read_lua_number(reader, endian)?);
    }

    let num_constant_functions = read_lua_int(reader, endian)? as usize;
    let mut constant_functions = Vec::with_capacity(num_constant_functions);
    for _ in 0..num_constant_functions {
        constant_functions.push(read_lua_function(reader, endian)?);
    }

    let num_instructions = read_lua_int(reader, endian)? as usize;
    let mut instructions = Vec::with_capacity(num_instructions);
    for _ in 0..num_instructions {
        instructions.push(Lua4Instruction::from_u32(read_lua_int(reader, endian)?)?);
    }

    if instructions.last() != Some(&Lua4Instruction::OP_END) {
        anyhow::bail!(
            "Expected function to end with OP_END, instead found: {:?}",
            instructions.last()
        );
    }

    Ok(Arc::new(Lua4Function {
        source,
        line,
        num_parameters,
        is_var_arg,
        max_stack_size,
        local_vars,
        line_infos,
        constant_strings,
        constant_numbers,
        constant_functions,
        instructions,
    }))
}
