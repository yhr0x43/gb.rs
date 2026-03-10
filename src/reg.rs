use core::cell::UnsafeCell;
use core::fmt;
use core::ptr;

#[repr(transparent)]
pub struct Reg<T> {
    value: UnsafeCell<T>,
}

impl<T> Reg<T> {
    pub const fn from_mut(t: &mut T) -> &mut Reg<T> {
        unsafe { &mut *(t as *mut T as *mut Reg<T>) }
    }

    pub fn set(&self, val: T) {
        unsafe { ptr::write_volatile(self.value.get(), val) }
    }

    pub fn replace(&self, val: T) -> T {
        unsafe {
            let old_val = ptr::read_volatile(self.value.get());
            self.set(val);
            old_val
        }
    }
}

impl<T: Copy> Reg<T> {
    pub fn get(&self) -> T {
        unsafe { *self.value.get() }
    }
}

impl<T: fmt::UpperHex + Copy> fmt::Debug for Reg<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{:X}", self.get())
    }
}

macro_rules! impl_reg_ops {
    ($($t:ty),* $(,)?) => {
        $(#[allow(unused)]
         impl Reg<$t> {
             pub fn inc(&self, other: $t) {
                 self.post_inc(other);
             }
             pub fn dec(&self, other: $t) {
                 self.post_dec(other);
             }
             pub fn pre_inc(&self, other: $t) -> $t {
                 let val = self.get().wrapping_add(other);
                 self.set(val);
                 val
             }
             pub fn post_inc(&self, other: $t) -> $t {
                 self.replace(self.get().wrapping_add(other))
             }
             pub fn pre_dec(&self, other: $t) -> $t {
                 let val = self.get().wrapping_sub(other);
                 self.set(val);
                 val
             }
             pub fn post_dec(&self, other: $t) -> $t {
                 self.replace(self.get().wrapping_sub(other))
             }
         })*
    };
}

impl_reg_ops!(u8, u16);

