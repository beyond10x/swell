//! Requirements of the host interface that are held by the compiler: each is a `compile_fail`
//! doc test on an item of its own, named after the requirement, and compiled only when the doc
//! tests are collected (`cfg(doctest)`). A `compile_fail` test passes on any compile error, and
//! stable rustdoc does not check the error code it names (measured: the test below passed with
//! `E0599` in place of `E0277`). So each one has a twin that compiles and differs from it in one
//! token only, which shows that the failure comes from that token.
//! (`docs/design/host-interface.md`, decisions 1 and 12.)

/// The static assertion of HOST-136: `InterruptHandle` is `Send` (and `Sync`, decision 12).
const _: fn() = || {
    fn send_and_sync<T: Send + Sync>() {}
    send_and_sync::<crate::InterruptHandle>();
};

/// HOST-136: an `Agent` is not `Send` (decision 1). Passing one to a function that requires
/// `Send` does not compile:
///
/// ```compile_fail,E0277
/// struct NoHooks;
/// impl swell::Host for NoHooks {}
/// fn requires<T: Send>(_: T) {}
/// fn give(agent: swell::Agent<NoHooks>) {
///     requires(agent);
/// }
/// ```
///
/// and the same code with `Sized` in place of `Send` compiles:
///
/// ```
/// struct NoHooks;
/// impl swell::Host for NoHooks {}
/// fn requires<T: Sized>(_: T) {}
/// fn give(agent: swell::Agent<NoHooks>) {
///     requires(agent);
/// }
/// ```
#[cfg(doctest)]
pub fn host_136_agent_is_not_send() {}
