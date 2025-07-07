use {
    assert_cmd::Command,
    assert_fs::TempDir,
    std::sync::{Mutex, MutexGuard, OnceLock},
    test_context::TestContext,
};

static SERIALIZE: OnceLock<Mutex<()>> = OnceLock::new();

#[allow(dead_code, reason = "The tests might not use all fields")]
pub struct OjoContext {
    pub ojo: String,
    pub temp_dir: TempDir,
    mutex_lock: MutexGuard<'static, ()>,
}

impl TestContext for OjoContext {
    fn setup() -> Self {
        let ojo = Command::cargo_bin("ojo")
            .unwrap()
            .get_program()
            .to_string_lossy()
            .to_string();

        let mutex_lock = SERIALIZE.get_or_init(|| Mutex::new(())).lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap(); //this is global for all tests! serializer helps

        Self {
            ojo,
            temp_dir, // Will drop temp dir automatically.
            mutex_lock,
        }
    }
}
