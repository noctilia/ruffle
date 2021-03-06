//! ActionScript Virtual Machine 2 (AS3) support

use crate::avm2::globals::SystemPrototypes;
use crate::avm2::method::Method;
use crate::avm2::script::{Script, TranslationUnit};
use crate::context::UpdateContext;
use crate::tag_utils::SwfSlice;
use gc_arena::{Collect, MutationContext};
use std::rc::Rc;
use swf::avm2::read::Reader;

#[macro_export]
macro_rules! avm_debug {
    ($avm: expr, $($arg:tt)*) => (
        if $avm.show_debug_output() {
            log::debug!($($arg)*)
        }
    )
}

mod activation;
mod array;
mod class;
mod domain;
mod events;
mod function;
mod globals;
mod method;
mod names;
mod object;
mod property;
mod property_map;
mod return_value;
mod scope;
mod script;
mod slot;
mod string;
mod traits;
mod value;

pub use crate::avm2::activation::Activation;
pub use crate::avm2::domain::Domain;
pub use crate::avm2::names::{Namespace, QName};
pub use crate::avm2::object::{Object, StageObject, TObject};
pub use crate::avm2::value::Value;

/// Boxed error alias.
///
/// As AVM2 is a far stricter VM than AVM1, this may eventually be replaced
/// with a proper Avm2Error enum.
pub type Error = Box<dyn std::error::Error>;

/// The state of an AVM2 interpreter.
#[derive(Collect)]
#[collect(no_drop)]
pub struct Avm2<'gc> {
    /// Values currently present on the operand stack.
    stack: Vec<Value<'gc>>,

    /// Global scope object.
    globals: Domain<'gc>,

    /// System prototypes.
    system_prototypes: Option<SystemPrototypes<'gc>>,

    #[cfg(feature = "avm_debug")]
    pub debug_output: bool,
}

impl<'gc> Avm2<'gc> {
    /// Construct a new AVM interpreter.
    pub fn new(mc: MutationContext<'gc, '_>) -> Self {
        let globals = Domain::global_domain(mc);

        Self {
            stack: Vec::new(),
            globals,
            system_prototypes: None,

            #[cfg(feature = "avm_debug")]
            debug_output: false,
        }
    }

    pub fn load_player_globals(context: &mut UpdateContext<'_, 'gc, '_>) -> Result<(), Error> {
        let globals = context.avm2.globals;
        let mut activation = Activation::from_nothing(context.reborrow());
        globals::load_player_globals(&mut activation, globals)
    }

    /// Return the current set of system prototypes.
    ///
    /// This function panics if the interpreter has not yet been initialized.
    pub fn prototypes(&self) -> &SystemPrototypes<'gc> {
        self.system_prototypes.as_ref().unwrap()
    }

    /// Run a script's initializer method.
    pub fn run_script_initializer(
        script: Script<'gc>,
        context: &mut UpdateContext<'_, 'gc, '_>,
    ) -> Result<(), Error> {
        let mut init_activation = Activation::from_script(context.reborrow(), script)?;

        let (method, scope) = script.init();
        match method {
            Method::Native(nf) => {
                nf(&mut init_activation, Some(scope), &[])?;
            }
            Method::Entry(_) => {
                init_activation.run_stack_frame_for_script(script)?;
            }
        };

        Ok(())
    }

    pub fn run_stack_frame_for_callable(
        callable: Object<'gc>,
        reciever: Option<Object<'gc>>,
        args: &[Value<'gc>],
        context: &mut UpdateContext<'_, 'gc, '_>,
    ) -> Result<(), Error> {
        let mut evt_activation = Activation::from_nothing(context.reborrow());
        callable.call(
            reciever,
            args,
            &mut evt_activation,
            reciever.and_then(|r| r.proto()),
        )?;

        Ok(())
    }

    /// Load an ABC file embedded in a `SwfSlice`.
    ///
    /// The `SwfSlice` must resolve to the contents of an ABC file.
    pub fn load_abc(
        abc: SwfSlice,
        _abc_name: &str,
        lazy_init: bool,
        context: &mut UpdateContext<'_, 'gc, '_>,
        domain: Domain<'gc>,
    ) -> Result<(), Error> {
        let mut read = Reader::new(abc.as_ref());

        let abc_file = Rc::new(read.read()?);
        let tunit = TranslationUnit::from_abc(abc_file.clone(), domain, context.gc_context);

        for i in (0..abc_file.scripts.len()).rev() {
            let mut script = tunit.load_script(i as u32, context.avm2, context.gc_context)?;

            if !lazy_init {
                script.globals(context)?;
            }
        }

        Ok(())
    }

    pub fn global_domain(&self) -> Domain<'gc> {
        self.globals
    }

    /// Push a value onto the operand stack.
    fn push(&mut self, value: impl Into<Value<'gc>>) {
        let value = value.into();
        avm_debug!(self, "Stack push {}: {:?}", self.stack.len(), value);
        self.stack.push(value);
    }

    /// Retrieve the top-most value on the operand stack.
    #[allow(clippy::let_and_return)]
    fn pop(&mut self) -> Value<'gc> {
        let value = self.stack.pop().unwrap_or_else(|| {
            log::warn!("Avm1::pop: Stack underflow");
            Value::Undefined
        });

        avm_debug!(self, "Stack pop {}: {:?}", self.stack.len(), value);

        value
    }

    fn pop_args(&mut self, arg_count: u32) -> Vec<Value<'gc>> {
        let mut args = Vec::with_capacity(arg_count as usize);
        args.resize(arg_count as usize, Value::Undefined);
        for arg in args.iter_mut().rev() {
            *arg = self.pop();
        }

        args
    }

    #[cfg(feature = "avm_debug")]
    #[inline]
    pub fn show_debug_output(&self) -> bool {
        self.debug_output
    }

    #[cfg(not(feature = "avm_debug"))]
    pub const fn show_debug_output(&self) -> bool {
        false
    }

    #[cfg(feature = "avm_debug")]
    pub fn set_show_debug_output(&mut self, visible: bool) {
        self.debug_output = visible;
    }

    #[cfg(not(feature = "avm_debug"))]
    pub const fn set_show_debug_output(&self, _visible: bool) {}
}
