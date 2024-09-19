use core::fmt::Write;
// pub struct MemoryMessage<T: Sized, const N: usize> {
//     data: [u8; N],
//     padding: [u8; 4096 - N],
//     /// This marker is used to determine how to transmit
//     _phantom: core::marker::PhantomData<T>,
// }

// struct PaddedMemoryMessage {
//     data: [u8; 64],
//     padding: [u8; 64],
// }

// impl<T, const N: usize> MemoryMessage<T, N>
// where
//     T: XousIpc,
// {
//     pub fn new() -> Self {
//         MemoryMessage {
//             data: [0; N],
//             _phantom: core::marker::PhantomData,
//         }
//     }

//     pub fn to_original(self) -> T {
//         assert!(core::mem::size_of::<T>() == N);
//         unsafe { *(self.data.as_ptr() as *const T)}
//     }
// }

/// An object is Sendable if it is guaranteed to be flat and contains no pointers.
/// This trait can be placed on objects that have invalid representations such as
/// bools (which can only be 0 or 1) but it is up to the implementer to ensure that
/// the correct object arrives on the other side.
pub unsafe trait Transmittable {}

pub trait XousIpc: Sized {
    // fn from_message<'a>(message: &mut MemoryMessage<Self, {core::mem::size_of::<Self>()}>) -> &'a mut Self where Self:Sized;

    // fn into_message(self) -> MemoryMessage<Self, core::mem::size_of::<Self>()> where Self:Sized;
}

unsafe impl Transmittable for u32 {}
unsafe impl Transmittable for u64 {}

pub struct String<const N: usize> {
    length: usize,
    buffer: [u8; N],
}

unsafe impl<const N: usize> Transmittable for String<N> {}

impl<const N: usize> String<N> {
    pub fn new() -> Self {
        String {
            buffer: [0; N],
            length: 0,
        }
    }
    pub fn from_str(s: &str) -> Self {
        let mut buffer = [0; N];
        let length = s.len();
        buffer.copy_from_slice(s.as_bytes());
        String { buffer, length }
    }
}

impl<const N: usize> core::fmt::Write for String<N> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        if s.len() + self.length > N {
            return Err(core::fmt::Error);
        }
        self.buffer[self.length..self.length + s.len()].copy_from_slice(s.as_bytes());
        self.length += s.len();
        Ok(())
    }
}

impl<const N: usize> core::fmt::Debug for String<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Safe because we guarantee the buffer is valid UTF-8
        write!(f, "{:?}", self.as_ref())
    }
}

impl<const N: usize> core::fmt::Display for String<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Safe because we guarantee the buffer is valid UTF-8
        write!(f, "{}", self.as_ref())
    }
}

impl<const N: usize> AsRef<str> for String<N> {
    fn as_ref(&self) -> &str {
        // Safe because we guarantee the buffer is valid UTF-8
        unsafe { core::str::from_utf8_unchecked(&self.buffer[0..self.length]) }
    }
}

#[test]
fn simple_ipc() {
    #[derive(flatipc_derive::XousIpc, Debug)]
    #[repr(C)]
    enum SimpleIpcEnum {
        Single(u32),
        Double(u32, u32),
        Triple {
            one: u32,
            two: u32,
            three: u32,
        },
        Quad {
            one: u32,
            two: u32,
            three: u32,
            four: u64,
            four_tuple: (u64, u64),
        },
    }

    #[derive(flatipc_derive::XousIpc, Debug)]
    #[repr(C)]
    struct SimpleIpcStruct {
        single: u32,
        double: (u32, u32),
        triple: (u32, u32, u32),
        quad: (u32, u32, u32, u64),
        sl: [u32; 4],
        nested_array: [[u32; 4]; 4],
        nothing: (),
        s: String<64>,
    }

    impl Default for SimpleIpcEnum {
        fn default() -> Self {
            SimpleIpcEnum::Single(0)
        }
    }

    let simple_ipc = SimpleIpcEnum::default();
    let mut s: String<64> = String::new();
    write!(&mut s, "Hello, world! s: {:?}", simple_ipc).unwrap();
    println!("String: {}", s);

    // println!("Simple IPC: {:?}", simple_ipc);
}

#[cfg(test)]
mod test;
