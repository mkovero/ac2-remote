//! Keep Wi-Fi multicast reception enabled only while the discovery worker exists.
use jni::{JValue, JavaVM, jni_sig, jni_str, objects::JObject, refs::Global};

pub fn landscape() -> Result<(), String> {
    let context = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(context.vm().cast()) };
    vm.attach_current_thread(|env| -> jni::errors::Result<()> {
        let raw_activity = context.context().cast();
        let activity = unsafe { env.as_cast_raw::<JObject>(&raw_activity)? };
        // SENSOR_LANDSCAPE permits either horizontal orientation.
        env.call_method(
            &*activity,
            jni_str!("setRequestedOrientation"),
            jni_sig!("(I)V"),
            &[JValue::Int(6)],
        )?;
        Ok(())
    })
    .map_err(|e| format!("Cannot select landscape orientation: {e}"))
}

pub struct MulticastLock {
    vm: JavaVM,
    lock: Global<JObject<'static>>,
}
impl MulticastLock {
    pub fn acquire() -> Result<Self, String> {
        let context = ndk_context::android_context();
        // android-activity initialized this VM and global Activity reference before android_main.
        let vm = unsafe { JavaVM::from_raw(context.vm().cast()) };
        let lock = vm
            .attach_current_thread(|env| -> jni::errors::Result<_> {
                let raw_activity = context.context().cast();
                // Borrow Android's global reference; never take ownership or delete it.
                let activity = unsafe { env.as_cast_raw::<JObject>(&raw_activity)? };
                let service = env.new_string("wifi")?;
                let manager = env
                    .call_method(
                        &*activity,
                        jni_str!("getSystemService"),
                        jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
                        &[JValue::Object(service.as_ref())],
                    )?
                    .l()?;
                let label = env.new_string("ac2-remote-discovery")?;
                let lock = env
                    .call_method(
                        &manager,
                        jni_str!("createMulticastLock"),
                        jni_sig!(
                            "(Ljava/lang/String;)Landroid/net/wifi/WifiManager$MulticastLock;"
                        ),
                        &[JValue::Object(label.as_ref())],
                    )?
                    .l()?;
                let global = env.new_global_ref(&lock)?;
                env.call_method(
                    &lock,
                    jni_str!("setReferenceCounted"),
                    jni_sig!("(Z)V"),
                    &[JValue::Bool(false)],
                )?;
                env.call_method(&lock, jni_str!("acquire"), jni_sig!("()V"), &[])?;
                Ok(global)
            })
            .map_err(|e| format!("Cannot enable Wi-Fi discovery: {e}"))?;
        Ok(Self { vm, lock })
    }
}
impl Drop for MulticastLock {
    fn drop(&mut self) {
        let _ = self
            .vm
            .attach_current_thread(|env| -> jni::errors::Result<()> {
                env.call_method(&self.lock, jni_str!("release"), jni_sig!("()V"), &[])?;
                Ok(())
            });
    }
}
