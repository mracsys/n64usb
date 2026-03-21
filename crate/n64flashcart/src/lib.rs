#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(dead_code)]
#[allow(unnecessary_transmutes)]
mod flashcart {
    use std::ptr;
    use std::os::raw::c_uchar;
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

    pub struct Header {
        pub datatype: USBDataType,
        pub length: usize,
    }

    pub fn initialize() {unsafe { device_initialize() }}
    pub fn find() -> DeviceError {unsafe { device_find() }}
    pub fn get_cart() -> CartType {unsafe { device_getcart() }}
    pub fn set_protocol(version: ProtocolVer) {unsafe { device_setprotocol(version); }}
    pub fn open() -> DeviceError {unsafe { device_open() }}
    pub fn close() -> DeviceError {unsafe { device_close() }}
    pub fn read() -> Result<(Header, Vec<u8>), DeviceError> {
        let mut raw_header: u32 = 0;
        let mut buff_ptr: *mut c_uchar = ptr::null_mut();
        let err = unsafe {
            device_receivedata(&mut raw_header, &mut buff_ptr)
        };
        if err != DeviceError::OK {
            return Err(err);
        }

        let header = Header {
            datatype: unsafe {std::mem::transmute(raw_header >> 24)},
            length: (raw_header & 0x00FFFFFF) as usize,
        };
        let mut data: Vec<u8> = vec![];
        if !buff_ptr.is_null() {
            data = unsafe {
                Vec::from_raw_parts(buff_ptr, header.length, header.length)
            };
        }

        Ok((header, data))
    }
    pub fn write(header: Header, data: Vec<u8>) -> DeviceError {
        unsafe {
            device_senddata(header.datatype, data.as_ptr() as *mut u8, header.length as u32)
        }
    }

    pub fn cart_type_to_str(cart: CartType) -> String {
        String::from(match cart {
            CartType::NONE => "None",
            CartType::_64DRIVE1 => "64Drive HW1",
            CartType::_64DRIVE2 => "64Drive HW2",
            CartType::EVERDRIVE => "Everdrive (X7 or V3)",
            CartType::SC64 => "Summercart64",
            CartType::GOPHER64 => "Gopher64",
            CartType::WII => "Wii",
        })
    }

    impl DeviceError {
        pub fn value(&self) -> u8 {
            *self as u8
        }
    }

    impl USBDataType {
        pub fn value(&self) -> u8 {
            *self as u8
        }
    }
}
pub use flashcart::*;
