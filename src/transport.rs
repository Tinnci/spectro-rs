//! Hardware abstraction layer for spectrometer communication.
//!
//! This module defines the [`Transport`] trait, which abstracts the underlying
//! communication protocol (USB, Bluetooth, Serial, etc.) from the device logic.

use crate::Result;
use std::time::Duration;

/// A trait abstracting low-level communication with a spectrometer device.
///
/// Implementors of this trait handle the raw byte-level I/O, allowing the
/// higher-level device logic (e.g., `Munki`) to be transport-agnostic.
///
/// # Examples
///
/// The primary implementation is [`UsbTransport`], which uses `rusb` for
/// USB HID communication. Future implementations could include:
/// - `BluetoothTransport` for BLE-enabled devices.
/// - `SerialTransport` for RS-232 serial port communication.
/// - `MockTransport` for unit testing without physical hardware.
pub trait Transport {
    /// Performs a control transfer read operation (Vendor IN).
    ///
    /// # Arguments
    /// * `request` - The bRequest field of the USB setup packet.
    /// * `value` - The wValue field.
    /// * `index` - The wIndex field.
    /// * `buf` - The buffer to read data into.
    /// * `timeout` - Maximum time to wait for the operation.
    ///
    /// # Returns
    /// The number of bytes actually read.
    fn control_read(
        &self,
        request: u8,
        value: u16,
        index: u16,
        buf: &mut [u8],
        timeout: Duration,
    ) -> Result<usize>;

    /// Performs a control transfer write operation (Vendor OUT).
    ///
    /// # Arguments
    /// * `request` - The bRequest field of the USB setup packet.
    /// * `value` - The wValue field.
    /// * `index` - The wIndex field.
    /// * `data` - The data to write.
    /// * `timeout` - Maximum time to wait for the operation.
    ///
    /// # Returns
    /// The number of bytes actually written.
    fn control_write(
        &self,
        request: u8,
        value: u16,
        index: u16,
        data: &[u8],
        timeout: Duration,
    ) -> Result<usize>;

    /// Reads data from an interrupt endpoint.
    ///
    /// # Arguments
    /// * `endpoint` - The endpoint address (e.g., 0x81 for EP1 IN).
    /// * `buf` - The buffer to read data into.
    /// * `timeout` - Maximum time to wait for data.
    ///
    /// # Returns
    /// The number of bytes actually read.
    fn interrupt_read(&self, endpoint: u8, buf: &mut [u8], timeout: Duration) -> Result<usize>;

    /// Returns a human-readable name for this transport (for debugging).
    fn name(&self) -> &str;
}

// ============================================================================
// USB Transport Implementation
// ============================================================================

use rusb::{DeviceHandle, UsbContext};

/// A USB-based transport implementation using `rusb`.
///
/// This is the standard transport for most X-Rite spectrometers connected via USB.
pub struct UsbTransport<T: UsbContext> {
    handle: DeviceHandle<T>,
}

impl<T: UsbContext> UsbTransport<T> {
    /// Creates a new `UsbTransport` from an open `rusb::DeviceHandle`.
    ///
    /// # Arguments
    /// * `handle` - An already-opened USB device handle with the interface claimed.
    pub fn new(handle: DeviceHandle<T>) -> Self {
        Self { handle }
    }

    /// Returns a reference to the underlying `rusb::DeviceHandle`.
    pub fn handle(&self) -> &DeviceHandle<T> {
        &self.handle
    }
}

impl<T: UsbContext> Transport for UsbTransport<T> {
    fn control_read(
        &self,
        request: u8,
        value: u16,
        index: u16,
        buf: &mut [u8],
        timeout: Duration,
    ) -> Result<usize> {
        const REQ_TYPE_VENDOR_IN: u8 = 0xC0;
        self.handle
            .read_control(REQ_TYPE_VENDOR_IN, request, value, index, buf, timeout)
            .map_err(crate::SpectroError::Usb)
    }

    fn control_write(
        &self,
        request: u8,
        value: u16,
        index: u16,
        data: &[u8],
        timeout: Duration,
    ) -> Result<usize> {
        const REQ_TYPE_VENDOR_OUT: u8 = 0x40;
        self.handle
            .write_control(REQ_TYPE_VENDOR_OUT, request, value, index, data, timeout)
            .map_err(crate::SpectroError::Usb)
    }

    fn interrupt_read(&self, endpoint: u8, buf: &mut [u8], timeout: Duration) -> Result<usize> {
        self.handle
            .read_interrupt(endpoint, buf, timeout)
            .map_err(crate::SpectroError::Usb)
    }

    fn name(&self) -> &str {
        "USB"
    }
}

// ============================================================================
// Mock Transport for Testing
// ============================================================================

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::cell::RefCell;
    use std::collections::VecDeque;

    /// A single control write entry: (request, value, index, data).
    pub type ControlWriteEntry = (u8, u16, u16, Vec<u8>);

    /// A mock transport for unit testing device logic without real hardware.
    ///
    /// You can pre-program responses that will be returned by subsequent reads.
    pub struct MockTransport {
        /// Queued responses for `control_read` calls.
        pub control_read_responses: RefCell<VecDeque<Vec<u8>>>,
        /// Queued responses for `interrupt_read` calls.
        pub interrupt_read_responses: RefCell<VecDeque<Vec<u8>>>,
        /// Log of all `control_write` calls for verification.
        pub control_write_log: RefCell<Vec<ControlWriteEntry>>,
    }

    impl MockTransport {
        pub fn new() -> Self {
            Self {
                control_read_responses: RefCell::new(VecDeque::new()),
                interrupt_read_responses: RefCell::new(VecDeque::new()),
                control_write_log: RefCell::new(Vec::new()),
            }
        }

        /// Queue a response to be returned by the next `control_read` call.
        pub fn queue_control_read(&self, data: Vec<u8>) {
            self.control_read_responses.borrow_mut().push_back(data);
        }

        /// Queue a response to be returned by the next `interrupt_read` call.
        pub fn queue_interrupt_read(&self, data: Vec<u8>) {
            self.interrupt_read_responses.borrow_mut().push_back(data);
        }
    }

    impl Default for MockTransport {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Transport for MockTransport {
        fn control_read(
            &self,
            _request: u8,
            _value: u16,
            _index: u16,
            buf: &mut [u8],
            _timeout: Duration,
        ) -> Result<usize> {
            let response = self
                .control_read_responses
                .borrow_mut()
                .pop_front()
                .unwrap_or_default();
            let len = response.len().min(buf.len());
            buf[..len].copy_from_slice(&response[..len]);
            Ok(len)
        }

        fn control_write(
            &self,
            request: u8,
            value: u16,
            index: u16,
            data: &[u8],
            _timeout: Duration,
        ) -> Result<usize> {
            self.control_write_log
                .borrow_mut()
                .push((request, value, index, data.to_vec()));
            Ok(data.len())
        }

        fn interrupt_read(
            &self,
            _endpoint: u8,
            buf: &mut [u8],
            _timeout: Duration,
        ) -> Result<usize> {
            let response = self
                .interrupt_read_responses
                .borrow_mut()
                .pop_front()
                .unwrap_or_default();
            let len = response.len().min(buf.len());
            buf[..len].copy_from_slice(&response[..len]);
            Ok(len)
        }

        fn name(&self) -> &str {
            "Mock"
        }
    }
}
