#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(dead_code)]
#[allow(unnecessary_transmutes)]
mod flashcart {
    use std::ptr;
    use std::os::raw::c_uchar;
    use std::ffi::{CStr, CString};
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));


    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct UsbSerialPort {
        vid: u16,
        pid: u16,
        serial: String,
        label: String,
    }

    impl Default for UsbSerialPort {
        fn default() -> Self {
            Self {
                vid: 0,
                pid: 0,
                serial: "".to_string(),
                label: "".to_string(),
            }
        }
    }

    impl ToString for UsbSerialPort {
        fn to_string(&self) -> String {
            self.label.clone()
        }
    }

    pub struct Header {
        pub datatype: USBDataType,
        pub length: usize,
    }

    pub fn initialize() {unsafe { device_initialize() }}
    pub fn find() -> DeviceError {unsafe { device_find() }}
    pub fn list() -> Vec<UsbSerialPort> {
        let mut count: u32 = 0;
        unsafe {
            device_list(std::ptr::null_mut(), &mut count);
        }
        if count == 0 {
            return Vec::new();
        }

        let mut devices: Vec<SerialDevice> = Vec::with_capacity(count as usize);
        unsafe {
            device_list(devices.as_mut_ptr(), &mut count);
            devices.set_len(count as usize);
        }
        devices.into_iter().map(|d| {
            let serial = CStr::from_bytes_until_nul(&d.serial.map(|b| b as u8)).unwrap_or_default().to_string_lossy().into_owned();
            let description = CStr::from_bytes_until_nul(&d.description.map(|b| b as u8)).unwrap_or_default().to_string_lossy().into_owned();
            let label = format!("{} ({:04X}:{:04X} {})", description, &d.vid, &d.pid, serial);
            UsbSerialPort { vid: d.vid, pid: d.pid, serial, label }
        }).collect()
    }
    pub fn connect(vid: u16, pid: u16, serial: &str) -> DeviceError {
        let id = ((vid as u32) << 16) | (pid as u32);
        let serial_ptr = CString::new(serial).expect("serial parameter must be valid string").into_raw();
        return unsafe {
            let err = device_connect(id, serial_ptr);
            // reclaim ownership
            let _ = CString::from_raw(serial_ptr);
            err
        }
    }
    pub fn get_cart() -> CartType {unsafe { device_getcart() }}
    pub fn set_protocol(version: ProtocolVer) {unsafe { device_setprotocol(version); }}
    pub fn get_protocol() {unsafe { device_getprotocol(); }}
    pub fn open() -> DeviceError {unsafe { device_open() }}
    pub fn isopen() -> bool {unsafe { device_isopen() }}
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
