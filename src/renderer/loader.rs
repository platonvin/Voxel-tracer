use std::{io::{Error, ErrorKind}, sync::Arc};
use std::{fs::File, io::{BufReader, Read}};

extern crate vulkano;
extern crate winit;
extern crate exr;

use vulkano::device::Device;
use vulkano::shader::ShaderModule;
use vulkano::shader::ShaderModuleCreateInfo;

pub fn load_file(filename: &str) -> Result<Vec<u32>, Error> {
    let mut data: Vec<u32> = Vec::new();

    let mut reader = BufReader::new(File::open(filename)?);
    loop {
        let mut buffer = [0; 4];
        let bytes_read = reader.read(&mut buffer)?;

        if bytes_read == 0 {
            break;
        }

        if bytes_read != 4 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Unexpected end of file",
            ));
        }

        let value = u32::from_le_bytes(buffer);
        data.push(value);
    }

    return Ok(data)
}

pub fn load_shader(device: Arc<Device>, name: &str) -> Arc<ShaderModule>{
    let buf = load_file(name).unwrap();
    let create_info = ShaderModuleCreateInfo::new(buf.as_slice());
    
unsafe {
    let sm = ShaderModule::new(device, create_info).unwrap();
    return sm;
}}