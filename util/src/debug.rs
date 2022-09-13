pub fn set_signal_handler() {
    use nix::sys::signal;
    extern "C" fn handle_sigsegv(_: i32) {
        panic!("signal::SIGSEGV {}", failure::Backtrace::new());
    }
    extern "C" fn handle_sig(s: i32) {
        panic!("signal {} {}", s, failure::Backtrace::new());
    }
    unsafe {
        signal::sigaction(signal::SIGILL, &signal::SigAction::new(signal::SigHandler::Handler(handle_sig), signal::SaFlags::SA_NODEFER, signal::SigSet::all()))
            .unwrap();
        signal::sigaction(
            signal::SIGSEGV,
            &signal::SigAction::new(signal::SigHandler::Handler(handle_sigsegv), signal::SaFlags::SA_NODEFER, signal::SigSet::empty()),
        )
        .unwrap();
    }
}
