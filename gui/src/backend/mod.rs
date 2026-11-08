//! Backend: real system interaction (drive detection, unison, qemu, apps).
//! Everything here is plain std + spawned CLI tools; the UI talks to it
//! through small structs and worker threads with mpsc channels.

#[allow(dead_code)]
pub mod apps;
#[allow(dead_code)]
pub mod drives;
#[allow(dead_code)]
pub mod unison;
