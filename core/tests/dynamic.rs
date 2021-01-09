pui_core::scalar_allocator! {
    thread_local struct ThreadLocal;
}

pui_core::scalar_allocator! {
    struct Global(u8);
}

#[test]
fn thread_local() {
    ThreadLocal::oneshot();
    std::panic::catch_unwind(ThreadLocal::oneshot).err().unwrap();
}

#[test]
fn global() {
    for _ in 0..255 {
        Global::oneshot();
    }
    std::panic::catch_unwind(Global::oneshot).err().unwrap();
}
