#[derive(Clone, Copy)]
pub struct Vec<const N: usize> {
    length: usize,
    buffer: [u8; N],
}

unsafe impl<const N: usize> crate::IpcSafe for Vec<N> {}
