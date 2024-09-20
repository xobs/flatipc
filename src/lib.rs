/// An object is Sendable if it is guaranteed to be flat and contains no pointers.
/// This trait can be placed on objects that have invalid representations such as
/// bools (which can only be 0 or 1) but it is up to the implementer to ensure that
/// the correct object arrives on the other side.
pub unsafe trait IpcSafe {}

pub mod string;
pub use string::String;

pub mod vec;
pub use vec::Vec;

pub trait Ipc: Sized {
    // fn from_message<'a>(message: &mut MemoryMessage<Self, {core::mem::size_of::<Self>()}>) -> &'a mut Self where Self:Sized;

    // fn into_message(self) -> MemoryMessage<Self, core::mem::size_of::<Self>()> where Self:Sized;
}

unsafe impl IpcSafe for i8 {}
unsafe impl IpcSafe for i16 {}
unsafe impl IpcSafe for i32 {}
unsafe impl IpcSafe for i64 {}
unsafe impl IpcSafe for i128 {}
unsafe impl IpcSafe for u8 {}
unsafe impl IpcSafe for u16 {}
unsafe impl IpcSafe for u32 {}
unsafe impl IpcSafe for u64 {}
unsafe impl IpcSafe for u128 {}
unsafe impl IpcSafe for bool {}
unsafe impl IpcSafe for usize {}
unsafe impl IpcSafe for isize {}
unsafe impl IpcSafe for char {}
unsafe impl<T, const N: usize> IpcSafe for [T; N] where T: IpcSafe {}
unsafe impl<T> IpcSafe for Option<T> where T: IpcSafe {}

pub trait MemoryMesage {
    /// What this memory message is a representation of.
    type Original;

    fn from_buffer<'a>(data: &'a [u8], signature: usize) -> Option<&'a Self>;
    unsafe fn from_buffer_unchecked<'a>(data: &'a [u8]) -> &'a Self;

    fn from_buffer_mut<'a>(data: &'a mut [u8], signature: usize) -> Option<&'a mut Self>;
    unsafe fn from_buffer_mut_unchecked<'a>(data: &'a mut [u8]) -> &'a mut Self;

    /// Return a reference to the original object while keeping the
    /// memory version alive.
    fn as_original(&self) -> &Self::Original;

    /// Return a reference to the original object while keeping the
    /// memory version alive.
    fn as_original_mut(&mut self) -> &mut Self::Original;

    /// Consume the memory version and return the original object.
    fn into_original(self) -> Self::Original;

    fn lend(&self, connection: usize, opcode: usize);

    fn lend_mut(&mut self, connection: usize, opcode: usize);

    /// Return the signature of this memory message. Useful for verifying
    /// that the correct message is being received.
    fn signature(&self) -> u32;
}

pub trait ToMemoryMessage {
    type Padded;
    fn into_message(self) -> Self::Padded;
}

#[cfg(test)]
mod test;
