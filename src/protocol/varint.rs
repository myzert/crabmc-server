use std::io::{self, Read, Write};

pub trait VarIntRead {
    fn read_varint(&mut self) -> io::Result<i32>;
    fn read_minecraft_string(&mut self) -> io::Result<String>;
}

pub trait VarIntWrite {
    fn write_varint(&mut self, val: i32) -> io::Result<()>;
    fn write_minecraft_string(&mut self, s: &str) -> io::Result<()>;
}

impl<T: Read> VarIntRead for T {
    fn read_varint(&mut self) -> io::Result<i32> {
        let mut num_read = 0;
        let mut result = 0i32;

        loop {
            let mut buf = [0u8; 1];
            self.read_exact(&mut buf)?;
            let read = buf[0];
            let value = (read & 0b0111_1111) as i32;
            result |= value << (7 * num_read);

            num_read += 1;
            if num_read > 5 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "VarInt is too big",
                ));
            }

            if (read & 0b1000_0000) == 0 {
                break;
            }
        }

        Ok(result)
    }

    fn read_minecraft_string(&mut self) -> io::Result<String> {
        let len = self.read_varint()? as usize;
        let mut buf = vec![0u8; len];
        self.read_exact(&mut buf)?;
        String::from_utf8(buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

impl<T: Write> VarIntWrite for T {
    fn write_varint(&mut self, mut value: i32) -> io::Result<()> {
        loop {
            let mut temp = (value & 0b0111_1111) as u8;
            value = ((value as u32) >> 7) as i32;
            if value != 0 {
                temp |= 0b1000_0000;
            }
            self.write_all(&[temp])?;
            if value == 0 {
                break;
            }
        }
        Ok(())
    }

    fn write_minecraft_string(&mut self, s: &str) -> io::Result<()> {
        let bytes = s.as_bytes();
        self.write_varint(bytes.len() as i32)?;
        self.write_all(bytes)
    }
}

#[allow(dead_code)]
pub fn varint_size(mut value: i32) -> usize {
    let mut size = 0;
    loop {
        size += 1;
        value = ((value as u32) >> 7) as i32;
        if value == 0 {
            break;
        }
    }
    size
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_varint_encode_decode() {
        let test_values = vec![0, 1, 2, 127, 128, 255, 25565, 2097151, -1, -2147483648];
        for val in test_values {
            let mut buf = Vec::new();
            buf.write_varint(val).unwrap();
            let mut cursor = Cursor::new(&buf);
            let decoded = cursor.read_varint().unwrap();
            assert_eq!(val, decoded, "Failed for value {}", val);
        }
    }

    #[test]
    fn test_minecraft_string() {
        let test_str = "CrabMC Server (Rust Edition)";
        let mut buf = Vec::new();
        buf.write_minecraft_string(test_str).unwrap();

        let mut cursor = Cursor::new(&buf);
        let decoded = cursor.read_minecraft_string().unwrap();
        assert_eq!(test_str, decoded);
    }
}
