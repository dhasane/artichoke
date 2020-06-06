use std::ffi::c_void;
use std::ptr;

use crate::convert::UnboxedValueGuard;
use crate::extn::core::array::Array;
use crate::extn::prelude::*;
use crate::gc::{MrbGarbageCollection, State as GcState};
use crate::sys;

impl BoxUnboxVmValue for Array {
    type Unboxed = Array;

    const RUBY_TYPE: &'static str = "Array";

    unsafe fn unbox_from_value<'a>(
        value: &'a mut Value,
        interp: &mut Artichoke,
    ) -> Result<UnboxedValueGuard<'a, Self::Unboxed>, Exception> {
        // Make sure we have a Data otherwise extraction will fail.
        if value.ruby_type() != Ruby::Data {
            let mut message = String::from("uninitialized ");
            message.push_str(Self::RUBY_TYPE);
            return Err(Exception::from(TypeError::new(interp, message)));
        }
        let state = interp.state.as_ref().ok_or(InterpreterExtractError)?;
        let spec = state
            .classes
            .get::<Self>()
            .ok_or_else(|| NotDefinedError::class(Self::RUBY_TYPE))?;
        // Sanity check that the RClass matches.
        let mrb = interp.mrb.as_mut();
        let mut rclass = spec
            .rclass(mrb)
            .ok_or_else(|| NotDefinedError::class(Self::RUBY_TYPE))?;
        if !ptr::eq(
            sys::mrb_sys_class_of_value(mrb, value.inner()),
            rclass.as_mut(),
        ) {
            let mut message = String::from("Could not extract ");
            message.push_str(Self::RUBY_TYPE);
            message.push_str(" from receiver");
            return Err(Exception::from(TypeError::new(interp, message)));
        }
        let ptr = sys::mrb_data_check_get_ptr(mrb, value.inner(), spec.data_type());
        if ptr.is_null() {
            // `Object#allocate` can be used to create `MRB_TT_DATA` without calling
            // `#initialize`. These objects will return a NULL pointer.
            let mut message = String::from("uninitialized ");
            message.push_str(Self::RUBY_TYPE);
            return Err(Exception::from(TypeError::new(interp, message)));
        }
        let value = Box::from_raw(ptr as *mut Self);
        // The underlying basic `ptr` and associated `free` function will be called by the GC.
        // The returned `Box` must not be stored.
        Ok(UnboxedValueGuard::new(value))
    }

    fn alloc_value(value: Self::Unboxed, interp: &mut Artichoke) -> Result<Value, Exception> {
        // Disable the GC to prevent a collection cycle from re-entering into
        // Rust code while the `State` is moved out of the `mrb`.
        //
        // It is not safe to run with the GC enabled in this method because:
        //
        // 1. This method must hold a borrow on the `State` to grab a handle to
        //    the class spec -> `sys::mrb_data_type`.
        // 2. Because of (1), the `State` must be moved into the `Artichoke`
        //    struct.
        // 3. Because of (2), subsequent mruby FFI calls will have a `NULL` ud
        //    pointer.
        // 4. Because of (3), it is not safe to re-enter into any Artichoke
        //    implemented FFI trampolines that expect to extract an interpreter.
        // 5. Garbage collection mark functions are one such trampoline that are
        //    not safe to re-enter.
        // 6. `Array` is implemented in Rust and implements its GC mark routine
        //    expecting to extract an intialized `Artichoke`.
        // 7. Failing to extract an initialized `Artichoke`, `Array` GC mark is
        //    a no-op.
        // 6. Values in these `Array`s are deallocated as unreachable, creating
        //    dangling references that when accessed result in a use-after-free.
        //
        // The most expedient way to avoid this is turn off the GC when
        // allocating with `mrb_data_object_alloc` below.
        let prior_gc_state = interp.disable_gc();

        let state = interp.state.as_ref().ok_or(InterpreterExtractError)?;
        let spec = state
            .classes
            .get::<Self>()
            .ok_or_else(|| NotDefinedError::class(Self::RUBY_TYPE))?;

        let data = Box::new(value);
        let ptr = Box::into_raw(data);

        let obj = unsafe {
            let mrb = interp.mrb.as_mut();
            let mut rclass = spec
                .rclass(mrb)
                .ok_or_else(|| NotDefinedError::class(Self::RUBY_TYPE))?;
            let alloc = sys::mrb_data_object_alloc(
                mrb,
                rclass.as_mut(),
                ptr as *mut c_void,
                spec.data_type(),
            );
            sys::mrb_sys_obj_value(alloc as *mut c_void)
        };

        if let GcState::Enabled = prior_gc_state {
            interp.enable_gc();
        }

        Ok(Value::from(obj))
    }

    fn box_into_value(
        value: Self::Unboxed,
        into: Value,
        interp: &mut Artichoke,
    ) -> Result<Value, Exception> {
        let state = interp.state.as_ref().ok_or(InterpreterExtractError)?;
        let spec = state
            .classes
            .get::<Self>()
            .ok_or_else(|| NotDefinedError::class(Self::RUBY_TYPE))?;

        let data = Box::new(value);
        let ptr = Box::into_raw(data);

        let mut obj = into.inner();

        unsafe {
            sys::mrb_sys_data_init(&mut obj, ptr as *mut c_void, spec.data_type());
        }
        Ok(Value::from(obj))
    }

    fn free(data: *mut c_void) {
        let data = data.cast::<*mut Self>();
        let unboxed = unsafe { Box::from_raw(data) };
        drop(unboxed);
    }
}
