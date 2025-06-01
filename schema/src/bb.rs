use std::borrow::Cow;
use std::f32;
use std::str;

/// A Kiwi byte buffer meant for reading.
///
/// Example usage:
///
/// ```
/// use std::borrow::Cow;
/// let mut bb = brine_kiwi_schema::ByteBuffer::new(&[240, 159, 141, 149, 0, 133, 242, 210, 237]);
/// assert_eq!(bb.read_string(), Ok(Cow::Borrowed("🍕")));
/// assert_eq!(bb.read_var_float(), Ok(123.456));
/// ```
///
pub struct ByteBuffer<'a> {
    data: &'a [u8],
    index: usize,
}

impl<'a> ByteBuffer<'a> {
    /// Create a new ByteBuffer that wraps the provided byte slice. The lifetime
    /// of the returned ByteBuffer must not outlive the lifetime of the byte
    /// slice.
    pub fn new(data: &[u8]) -> ByteBuffer {
        ByteBuffer { data, index: 0 }
    }

    /// Retrieves the underlying byte slice.
    pub fn data(&self) -> &'a [u8] {
        self.data
    }

    /// Retrieves the current index into the underlying byte slice. This starts
    /// off as 0 and ends up as `self.data().len()` when everything has been
    /// read.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Try to read a boolean value starting at the current index.
    pub fn read_bool(&mut self) -> Result<bool, ()> {
        match self.read_byte() {
            Ok(0) => Ok(false),
            Ok(1) => Ok(true),
            _ => Err(()),
        }
    }

    /// Try to read a byte starting at the current index.
    pub fn read_byte(&mut self) -> Result<u8, ()> {
        if self.index >= self.data.len() {
            Err(())
        } else {
            let value = self.data[self.index];
            self.index = self.index + 1;
            Ok(value)
        }
    }

    /// Try to read a byte starting at the current index.
    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], ()> {
        if self.index + len > self.data.len() {
            Err(())
        } else {
            let value = &self.data[self.index..self.index + len];
            self.index = self.index + len;
            Ok(value)
        }
    }

    /// Try to read a variable-length signed 32-bit integer starting at the
    /// current index.
    pub fn read_var_int(&mut self) -> Result<i32, ()> {
        let value = self.read_var_uint()?;
        Ok((if (value & 1) != 0 {
            !(value >> 1)
        } else {
            value >> 1
        }) as i32)
    }

    /// Try to read a variable-length unsigned 32-bit integer starting at the
    /// current index.
    pub fn read_var_uint(&mut self) -> Result<u32, ()> {
        let mut shift: u8 = 0;
        let mut result: u32 = 0;

        loop {
            let byte = self.read_byte()?;
            result |= ((byte & 127) as u32) << shift;
            shift += 7;

            if (byte & 128) == 0 || shift >= 35 {
                break;
            }
        }

        Ok(result)
    }

    /// Try to read a variable-length 32-bit floating-point number starting at
    /// the current index.
    pub fn read_var_float(&mut self) -> Result<f32, ()> {
        let first = self.read_byte()?;

        // Optimization: use a single byte to store zero
        if first == 0 {
            Ok(0.0)
        } else if self.index + 3 > self.data.len() {
            Err(())
        }
        // Endian-independent 32-bit read
        else {
            let mut bits: u32 = first as u32
                | ((self.data[self.index] as u32) << 8)
                | ((self.data[self.index + 1] as u32) << 16)
                | ((self.data[self.index + 2] as u32) << 24);
            self.index += 3;

            // Move the exponent back into place
            bits = (bits << 23) | (bits >> 9);

            Ok(f32::from_bits(bits))
        }
    }

    /// Try to read a UTF-8 string starting at the current index. This string is
    /// returned as a slice so it just aliases the underlying memory.
    pub fn read_string(&mut self) -> Result<Cow<'a, str>, ()> {
        let start = self.index;

        while self.index < self.data.len() {
            if self.data[self.index] == 0 {
                self.index += 1;
                return Ok(String::from_utf8_lossy(&self.data[start..self.index - 1]));
            }

            self.index += 1;
        }

        Err(())
    }

    /// Try to read a variable-length signed 64-bit integer starting at the
    /// current index.
    pub fn read_var_int64(&mut self) -> Result<i64, ()> {
        let value = self.read_var_uint64()?;
        Ok((if (value & 1) != 0 {
            !(value >> 1)
        } else {
            value >> 1
        }) as i64)
    }

    /// Try to read a variable-length unsigned 64-bit integer starting at the
    /// current index.
    pub fn read_var_uint64(&mut self) -> Result<u64, ()> {
        let mut shift: u8 = 0;
        let mut result: u64 = 0;

        loop {
            let byte = self.read_byte()?;
            if (byte & 128) == 0 || shift >= 56 {
                result |= (byte as u64) << shift;
                break;
            }
            result |= ((byte & 127) as u64) << shift;
            shift += 7;
        }

        Ok(result)
    }
}

#[test]
fn read_bool() {
    let read = |bytes| ByteBuffer::new(bytes).read_bool();
    assert_eq!(read(&[]), Err(()));
    assert_eq!(read(&[0]), Ok(false));
    assert_eq!(read(&[1]), Ok(true));
    assert_eq!(read(&[2]), Err(()));
}

#[test]
fn read_byte() {
    let read = |bytes| ByteBuffer::new(bytes).read_byte();
    assert_eq!(read(&[]), Err(()));
    assert_eq!(read(&[0]), Ok(0));
    assert_eq!(read(&[1]), Ok(1));
    assert_eq!(read(&[254]), Ok(254));
    assert_eq!(read(&[255]), Ok(255));
}

#[test]
fn read_bytes() {
    let read = |bytes, len| ByteBuffer::new(bytes).read_bytes(len);
    assert_eq!(read(&[], 0), Ok(vec![].as_slice()));
    assert_eq!(read(&[], 1), Err(()));
    assert_eq!(read(&[0], 0), Ok(vec![].as_slice()));
    assert_eq!(read(&[0], 1), Ok(vec![0].as_slice()));
    assert_eq!(read(&[0], 2), Err(()));

    let mut bb = ByteBuffer::new(&[1, 2, 3, 4, 5]);
    assert_eq!(bb.read_bytes(3), Ok(vec![1, 2, 3].as_slice()));
    assert_eq!(bb.read_bytes(2), Ok(vec![4, 5].as_slice()));
    assert_eq!(bb.read_bytes(1), Err(()));
}

#[test]
fn read_var_int() {
    let read = |bytes| ByteBuffer::new(bytes).read_var_int();
    assert_eq!(read(&[]), Err(()));
    assert_eq!(read(&[0]), Ok(0));
    assert_eq!(read(&[1]), Ok(-1));
    assert_eq!(read(&[2]), Ok(1));
    assert_eq!(read(&[3]), Ok(-2));
    assert_eq!(read(&[4]), Ok(2));
    assert_eq!(read(&[127]), Ok(-64));
    assert_eq!(read(&[128]), Err(()));
    assert_eq!(read(&[128, 0]), Ok(0));
    assert_eq!(read(&[128, 1]), Ok(64));
    assert_eq!(read(&[128, 2]), Ok(128));
    assert_eq!(read(&[129, 0]), Ok(-1));
    assert_eq!(read(&[129, 1]), Ok(-65));
    assert_eq!(read(&[129, 2]), Ok(-129));
    assert_eq!(read(&[253, 255, 7]), Ok(-65535));
    assert_eq!(read(&[254, 255, 7]), Ok(65535));
    assert_eq!(read(&[253, 255, 255, 255, 15]), Ok(-2147483647));
    assert_eq!(read(&[254, 255, 255, 255, 15]), Ok(2147483647));
    assert_eq!(read(&[255, 255, 255, 255, 15]), Ok(-2147483648));
}

#[test]
fn read_var_uint() {
    let read = |bytes| ByteBuffer::new(bytes).read_var_uint();
    assert_eq!(read(&[]), Err(()));
    assert_eq!(read(&[0]), Ok(0));
    assert_eq!(read(&[1]), Ok(1));
    assert_eq!(read(&[2]), Ok(2));
    assert_eq!(read(&[3]), Ok(3));
    assert_eq!(read(&[4]), Ok(4));
    assert_eq!(read(&[127]), Ok(127));
    assert_eq!(read(&[128]), Err(()));
    assert_eq!(read(&[128, 0]), Ok(0));
    assert_eq!(read(&[128, 1]), Ok(128));
    assert_eq!(read(&[128, 2]), Ok(256));
    assert_eq!(read(&[129, 0]), Ok(1));
    assert_eq!(read(&[129, 1]), Ok(129));
    assert_eq!(read(&[129, 2]), Ok(257));
    assert_eq!(read(&[253, 255, 7]), Ok(131069));
    assert_eq!(read(&[254, 255, 7]), Ok(131070));
    assert_eq!(read(&[253, 255, 255, 255, 15]), Ok(4294967293));
    assert_eq!(read(&[254, 255, 255, 255, 15]), Ok(4294967294));
    assert_eq!(read(&[255, 255, 255, 255, 15]), Ok(4294967295));
}

#[test]
fn read_var_float() {
    let read = |bytes| ByteBuffer::new(bytes).read_var_float();
    assert_eq!(read(&[]), Err(()));
    assert_eq!(read(&[0]), Ok(0.0));
    assert_eq!(read(&[133, 242, 210, 237]), Ok(123.456));
    assert_eq!(read(&[133, 243, 210, 237]), Ok(-123.456));
    assert_eq!(read(&[254, 255, 255, 255]), Ok(f32::MIN));
    assert_eq!(read(&[254, 254, 255, 255]), Ok(f32::MAX));
    assert_eq!(read(&[1, 1, 0, 0]), Ok(-f32::MIN_POSITIVE));
    assert_eq!(read(&[1, 0, 0, 0]), Ok(f32::MIN_POSITIVE));
    assert_eq!(read(&[255, 1, 0, 0]), Ok(f32::NEG_INFINITY));
    assert_eq!(read(&[255, 0, 0, 0]), Ok(f32::INFINITY));
    assert_eq!(read(&[255, 0, 0, 128]).map(|f| f.is_nan()), Ok(true));
}

#[test]
fn read_string() {
    let read = |bytes| ByteBuffer::new(bytes).read_string();
    assert_eq!(read(&[]), Err(()));
    assert_eq!(read(&[0]), Ok(Cow::Borrowed("")));
    assert_eq!(read(&[97]), Err(()));
    assert_eq!(read(&[97, 0]), Ok(Cow::Borrowed("a")));
    assert_eq!(read(&[97, 98, 99, 0]), Ok(Cow::Borrowed("abc")));
    assert_eq!(read(&[240, 159, 141, 149, 0]), Ok(Cow::Borrowed("🍕")));
    assert_eq!(
        read(&[97, 237, 160, 188, 99, 0]),
        Ok(Cow::Owned("a���c".to_owned()))
    );
}

#[test]
fn read_var_int64() {
    let read = |bytes| ByteBuffer::new(bytes).read_var_int64();
    assert_eq!(read(&[]), Err(()));
    assert_eq!(read(&[0]), Ok(0));
    assert_eq!(read(&[1]), Ok(-1));
    assert_eq!(read(&[2]), Ok(1));
    assert_eq!(read(&[3]), Ok(-2));
    assert_eq!(read(&[4]), Ok(2));
    assert_eq!(read(&[127]), Ok(-64));
    assert_eq!(read(&[128]), Err(()));
    assert_eq!(read(&[128, 0]), Ok(0));
    assert_eq!(read(&[128, 1]), Ok(64));
    assert_eq!(read(&[128, 2]), Ok(128));
    assert_eq!(read(&[129, 0]), Ok(-1));
    assert_eq!(read(&[129, 1]), Ok(-65));
    assert_eq!(read(&[129, 2]), Ok(-129));
    assert_eq!(read(&[253, 255, 7]), Ok(-65535));
    assert_eq!(read(&[254, 255, 7]), Ok(65535));
    assert_eq!(read(&[253, 255, 255, 255, 15]), Ok(-2147483647));
    assert_eq!(read(&[254, 255, 255, 255, 15]), Ok(2147483647));
    assert_eq!(read(&[255, 255, 255, 255, 15]), Ok(-2147483648));
    assert_eq!(
        read(&[0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88]),
        Ok(0x4407_0C14_2030_4040)
    );
    assert_eq!(
        read(&[0x81, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x20]),
        Ok(-0x1000_0000_0000_0001)
    );
    assert_eq!(
        read(&[0x82, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x20]),
        Ok(0x1000_0000_0000_0001)
    );
    assert_eq!(
        read(&[0xFD, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F]),
        Ok(-0x3FFF_FFFF_FFFF_FFFF)
    );
    assert_eq!(
        read(&[0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F]),
        Ok(0x3FFF_FFFF_FFFF_FFFF)
    );
    assert_eq!(
        read(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F]),
        Ok(-0x4000_0000_0000_0000)
    );
    assert_eq!(
        read(&[0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80]),
        Ok(0x4000_0000_0000_0000)
    );
    assert_eq!(
        read(&[0xFD, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]),
        Ok(-0x7FFF_FFFF_FFFF_FFFF)
    );
    assert_eq!(
        read(&[0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]),
        Ok(0x7FFF_FFFF_FFFF_FFFF)
    );
    assert_eq!(
        read(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]),
        Ok(-0x8000_0000_0000_0000)
    );
}

#[test]
fn read_var_uint64() {
    let read = |bytes| ByteBuffer::new(bytes).read_var_uint64();
    assert_eq!(read(&[]), Err(()));
    assert_eq!(read(&[0]), Ok(0));
    assert_eq!(read(&[1]), Ok(1));
    assert_eq!(read(&[2]), Ok(2));
    assert_eq!(read(&[3]), Ok(3));
    assert_eq!(read(&[4]), Ok(4));
    assert_eq!(read(&[127]), Ok(127));
    assert_eq!(read(&[128]), Err(()));
    assert_eq!(read(&[128, 0]), Ok(0));
    assert_eq!(read(&[128, 1]), Ok(128));
    assert_eq!(read(&[128, 2]), Ok(256));
    assert_eq!(read(&[129, 0]), Ok(1));
    assert_eq!(read(&[129, 1]), Ok(129));
    assert_eq!(read(&[129, 2]), Ok(257));
    assert_eq!(read(&[253, 255, 7]), Ok(131069));
    assert_eq!(read(&[254, 255, 7]), Ok(131070));
    assert_eq!(read(&[253, 255, 255, 255, 15]), Ok(4294967293));
    assert_eq!(read(&[254, 255, 255, 255, 15]), Ok(4294967294));
    assert_eq!(read(&[255, 255, 255, 255, 15]), Ok(4294967295));
    assert_eq!(
        read(&[0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88]),
        Ok(0x880E_1828_4060_8080)
    );
    assert_eq!(
        read(&[0x81, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x10]),
        Ok(0x1000_0000_0000_0001)
    );
    assert_eq!(
        read(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F]),
        Ok(0x7FFF_FFFF_FFFF_FFFF)
    );
    assert_eq!(
        read(&[0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80]),
        Ok(0x8000_0000_0000_0000)
    );
    assert_eq!(
        read(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]),
        Ok(0xFFFF_FFFF_FFFF_FFFF)
    );
}

#[test]
fn read_sequence() {
    let mut bb = ByteBuffer::new(&[
        0, 133, 242, 210, 237, 240, 159, 141, 149, 0, 149, 154, 239, 58,
    ]);
    assert_eq!(bb.read_var_float(), Ok(0.0));
    assert_eq!(bb.read_var_float(), Ok(123.456));
    assert_eq!(bb.read_string(), Ok(Cow::Borrowed("🍕")));
    assert_eq!(bb.read_var_uint(), Ok(123456789));
}

/// A Kiwi byte buffer meant for writing.
///
/// Example usage:
///
/// ```
/// let mut bb = brine_kiwi_schema::ByteBufferMut::new();
/// bb.write_string("🍕");
/// bb.write_var_float(123.456);
/// assert_eq!(bb.data(), [240, 159, 141, 149, 0, 133, 242, 210, 237]);
/// ```
///
pub struct ByteBufferMut {
    data: Vec<u8>,
}

impl ByteBufferMut {
    /// Creates an empty ByteBufferMut ready for writing.
    pub fn new() -> ByteBufferMut {
        ByteBufferMut { data: vec![] }
    }

    /// Consumes this buffer and returns the underlying backing store. Use this
    /// to get the data out when you're done writing to the buffer.
    pub fn data(self) -> Vec<u8> {
        self.data
    }

    /// Returns the number of bytes written so far.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Write a boolean value to the end of the buffer.
    pub fn write_bool(&mut self, value: bool) {
        self.data.push(if value { 1 } else { 0 });
    }

    /// Write a byte to the end of the buffer.
    pub fn write_byte(&mut self, value: u8) {
        self.data.push(value);
    }

    /// Write a raw byte slice to the end of the buffer.
    pub fn write_bytes(&mut self, value: &[u8]) {
        self.data.extend_from_slice(value);
    }

    /// Write a variable-length signed 32-bit integer to the end of the buffer.
    pub fn write_var_int(&mut self, value: i32) {
        self.write_var_uint(((value << 1) ^ (value >> 31)) as u32);
    }

    /// Write a variable-length unsigned 32-bit integer to the end of the buffer.
    pub fn write_var_uint(&mut self, mut value: u32) {
        loop {
            let byte = value as u8 & 127;
            value >>= 7;

            if value == 0 {
                self.write_byte(byte);
                return;
            }

            self.write_byte(byte | 128);
        }
    }

    /// Write a variable-length 32-bit floating-point number to the end of the
    /// buffer.
    pub fn write_var_float(&mut self, value: f32) {
        // Reinterpret as an integer
        let mut bits = value.to_bits();

        // Move the exponent to the first 8 bits
        bits = (bits >> 23) | (bits << 9);

        // Optimization: use a single byte to store zero and denormals (try for an exponent of 0)
        if (bits & 255) == 0 {
            self.data.push(0);
            return;
        }

        // Endian-independent 32-bit write
        self.data.extend_from_slice(&[
            bits as u8,
            (bits >> 8) as u8,
            (bits >> 16) as u8,
            (bits >> 24) as u8,
        ]);
    }

    /// Write a UTF-8 string to the end of the buffer.
    pub fn write_string(&mut self, value: &str) {
        self.data.extend_from_slice(value.as_bytes());
        self.data.push(0);
    }

    /// Write a variable-length signed 64-bit integer to the end of the buffer.
    pub fn write_var_int64(&mut self, value: i64) {
        self.write_var_uint64(((value << 1) ^ (value >> 63)) as u64);
    }

    /// Write a variable-length unsigned 64-bit integer to the end of the buffer.
    pub fn write_var_uint64(&mut self, mut value: u64) {
        let mut i = 0;
        while value > 127 && i < 8 {
            self.write_byte((value as u8 & 127) | 128);
            value >>= 7;
            i += 1;
        }
        self.write_byte(value as u8);
    }
}

#[cfg(test)]
fn write_once(cb: fn(&mut ByteBufferMut)) -> Vec<u8> {
    let mut bb = ByteBufferMut::new();
    cb(&mut bb);
    bb.data()
}

#[test]
fn write_bool() {
    assert_eq!(write_once(|bb| bb.write_bool(false)), [0]);
    assert_eq!(write_once(|bb| bb.write_bool(true)), [1]);
}

#[test]
fn write_byte() {
    assert_eq!(write_once(|bb| bb.write_byte(0)), [0]);
    assert_eq!(write_once(|bb| bb.write_byte(1)), [1]);
    assert_eq!(write_once(|bb| bb.write_byte(254)), [254]);
    assert_eq!(write_once(|bb| bb.write_byte(255)), [255]);
}

#[test]
fn write_bytes() {
    let mut bb = ByteBufferMut::new();
    bb.write_bytes(&[1, 2, 3]);
    bb.write_bytes(&[]);
    bb.write_bytes(&[4, 5]);
    assert_eq!(bb.data(), [1, 2, 3, 4, 5]);
}

#[test]
fn write_var_int() {
    assert_eq!(write_once(|bb| bb.write_var_int(0)), [0]);
    assert_eq!(write_once(|bb| bb.write_var_int(-1)), [1]);
    assert_eq!(write_once(|bb| bb.write_var_int(1)), [2]);
    assert_eq!(write_once(|bb| bb.write_var_int(-2)), [3]);
    assert_eq!(write_once(|bb| bb.write_var_int(2)), [4]);
    assert_eq!(write_once(|bb| bb.write_var_int(-64)), [127]);
    assert_eq!(write_once(|bb| bb.write_var_int(64)), [128, 1]);
    assert_eq!(write_once(|bb| bb.write_var_int(128)), [128, 2]);
    assert_eq!(write_once(|bb| bb.write_var_int(-129)), [129, 2]);
    assert_eq!(write_once(|bb| bb.write_var_int(-65535)), [253, 255, 7]);
    assert_eq!(write_once(|bb| bb.write_var_int(65535)), [254, 255, 7]);
    assert_eq!(
        write_once(|bb| bb.write_var_int(-2147483647)),
        [253, 255, 255, 255, 15]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_int(2147483647)),
        [254, 255, 255, 255, 15]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_int(-2147483648)),
        [255, 255, 255, 255, 15]
    );
}

#[test]
fn write_var_uint() {
    assert_eq!(write_once(|bb| bb.write_var_uint(0)), [0]);
    assert_eq!(write_once(|bb| bb.write_var_uint(1)), [1]);
    assert_eq!(write_once(|bb| bb.write_var_uint(2)), [2]);
    assert_eq!(write_once(|bb| bb.write_var_uint(3)), [3]);
    assert_eq!(write_once(|bb| bb.write_var_uint(4)), [4]);
    assert_eq!(write_once(|bb| bb.write_var_uint(127)), [127]);
    assert_eq!(write_once(|bb| bb.write_var_uint(128)), [128, 1]);
    assert_eq!(write_once(|bb| bb.write_var_uint(256)), [128, 2]);
    assert_eq!(write_once(|bb| bb.write_var_uint(129)), [129, 1]);
    assert_eq!(write_once(|bb| bb.write_var_uint(257)), [129, 2]);
    assert_eq!(write_once(|bb| bb.write_var_uint(131069)), [253, 255, 7]);
    assert_eq!(write_once(|bb| bb.write_var_uint(131070)), [254, 255, 7]);
    assert_eq!(
        write_once(|bb| bb.write_var_uint(4294967293)),
        [253, 255, 255, 255, 15]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_uint(4294967294)),
        [254, 255, 255, 255, 15]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_uint(4294967295)),
        [255, 255, 255, 255, 15]
    );
}

#[test]
fn write_var_float() {
    assert_eq!(write_once(|bb| bb.write_var_float(0.0)), [0]);
    assert_eq!(write_once(|bb| bb.write_var_float(-0.0)), [0]);
    assert_eq!(
        write_once(|bb| bb.write_var_float(123.456)),
        [133, 242, 210, 237]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_float(-123.456)),
        [133, 243, 210, 237]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_float(f32::MIN)),
        [254, 255, 255, 255]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_float(f32::MAX)),
        [254, 254, 255, 255]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_float(-f32::MIN_POSITIVE)),
        [1, 1, 0, 0]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_float(f32::MIN_POSITIVE)),
        [1, 0, 0, 0]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_float(f32::NEG_INFINITY)),
        [255, 1, 0, 0]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_float(f32::INFINITY)),
        [255, 0, 0, 0]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_float(f32::NAN)),
        [255, 0, 0, 128]
    );
    assert_eq!(write_once(|bb| bb.write_var_float(1.0e-40)), [0]);
}

#[test]
fn write_string() {
    assert_eq!(write_once(|bb| bb.write_string("")), [0]);
    assert_eq!(write_once(|bb| bb.write_string("a")), [97, 0]);
    assert_eq!(write_once(|bb| bb.write_string("abc")), [97, 98, 99, 0]);
    assert_eq!(
        write_once(|bb| bb.write_string("🍕")),
        [240, 159, 141, 149, 0]
    );
}

#[test]
fn write_var_int64() {
    assert_eq!(write_once(|bb| bb.write_var_int64(0)), [0]);
    assert_eq!(write_once(|bb| bb.write_var_int64(-1)), [1]);
    assert_eq!(write_once(|bb| bb.write_var_int64(1)), [2]);
    assert_eq!(write_once(|bb| bb.write_var_int64(-2)), [3]);
    assert_eq!(write_once(|bb| bb.write_var_int64(2)), [4]);
    assert_eq!(write_once(|bb| bb.write_var_int64(-64)), [127]);
    assert_eq!(write_once(|bb| bb.write_var_int64(64)), [128, 1]);
    assert_eq!(write_once(|bb| bb.write_var_int64(128)), [128, 2]);
    assert_eq!(write_once(|bb| bb.write_var_int64(-129)), [129, 2]);
    assert_eq!(write_once(|bb| bb.write_var_int64(-65535)), [253, 255, 7]);
    assert_eq!(write_once(|bb| bb.write_var_int64(65535)), [254, 255, 7]);
    assert_eq!(
        write_once(|bb| bb.write_var_int64(-2147483647)),
        [253, 255, 255, 255, 15]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_int64(2147483647)),
        [254, 255, 255, 255, 15]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_int64(-2147483648)),
        [255, 255, 255, 255, 15]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_int64(-0x1000_0000_0000_0001)),
        [0x81, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x20]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_int64(0x1000_0000_0000_0001)),
        [0x82, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x20]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_int64(-0x3FFF_FFFF_FFFF_FFFF)),
        [0xFD, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_int64(0x3FFF_FFFF_FFFF_FFFF)),
        [0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_int64(-0x4000_0000_0000_0000)),
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_int64(0x4000_0000_0000_0000)),
        [0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_int64(-0x7FFF_FFFF_FFFF_FFFF)),
        [0xFD, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_int64(0x7FFF_FFFF_FFFF_FFFF)),
        [0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_int64(-0x8000_0000_0000_0000)),
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
    );
}

#[test]
fn write_var_uint64() {
    assert_eq!(write_once(|bb| bb.write_var_uint64(0)), [0]);
    assert_eq!(write_once(|bb| bb.write_var_uint64(1)), [1]);
    assert_eq!(write_once(|bb| bb.write_var_uint64(2)), [2]);
    assert_eq!(write_once(|bb| bb.write_var_uint64(3)), [3]);
    assert_eq!(write_once(|bb| bb.write_var_uint64(4)), [4]);
    assert_eq!(write_once(|bb| bb.write_var_uint64(127)), [127]);
    assert_eq!(write_once(|bb| bb.write_var_uint64(128)), [128, 1]);
    assert_eq!(write_once(|bb| bb.write_var_uint64(256)), [128, 2]);
    assert_eq!(write_once(|bb| bb.write_var_uint64(129)), [129, 1]);
    assert_eq!(write_once(|bb| bb.write_var_uint64(257)), [129, 2]);
    assert_eq!(write_once(|bb| bb.write_var_uint64(131069)), [253, 255, 7]);
    assert_eq!(write_once(|bb| bb.write_var_uint64(131070)), [254, 255, 7]);
    assert_eq!(
        write_once(|bb| bb.write_var_uint64(4294967293)),
        [253, 255, 255, 255, 15]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_uint64(4294967294)),
        [254, 255, 255, 255, 15]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_uint64(4294967295)),
        [255, 255, 255, 255, 15]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_uint64(0x1000_0000_0000_0001)),
        [0x81, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x10]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_uint64(0x7FFF_FFFF_FFFF_FFFF)),
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_uint64(0x8000_0000_0000_0000)),
        [0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80]
    );
    assert_eq!(
        write_once(|bb| bb.write_var_uint64(0xFFFF_FFFF_FFFF_FFFF)),
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]
    );
}

#[test]
fn write_sequence() {
    let mut bb = ByteBufferMut::new();
    bb.write_var_float(0.0);
    bb.write_var_float(123.456);
    bb.write_string("🍕");
    bb.write_var_uint(123456789);
    assert_eq!(
        bb.data(),
        [0, 133, 242, 210, 237, 240, 159, 141, 149, 0, 149, 154, 239, 58]
    );
}
