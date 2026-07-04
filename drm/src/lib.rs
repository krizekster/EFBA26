use aes_gcm::{
    aead::Aead, KeyInit,
    Aes256Gcm, Nonce,
};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::Rng;
use std::sync::atomic::{AtomicBool, Ordering};

pub static TAMPER_FLAG: AtomicBool = AtomicBool::new(false);
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::fs::OpenOptions;
use std::io::Write;

pub fn write_audit_log(event_type: &str, msg: &str) {
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("security_audit.log") {
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
        let _ = writeln!(file, "[{}] [{}] {}", ts, event_type, msg);
    }
}

pub trait ProtectionProfile: Send {
    fn on_startup(&mut self) -> Result<(), &'static str>;
    fn on_checkpoint(&mut self) -> Result<(), &'static str>;
    fn load_protected_asset(&mut self) -> Result<Vec<u8>, String>;
    fn check_tamper(&self) -> bool { false }
}

pub struct Baseline;
pub struct AlwaysOnline;
pub struct VMAntitamper;

impl ProtectionProfile for Baseline {
    fn on_startup(&mut self) -> Result<(), &'static str> { Ok(()) }
    fn on_checkpoint(&mut self) -> Result<(), &'static str> { Ok(()) }
    fn load_protected_asset(&mut self) -> Result<Vec<u8>, String> {
        Ok(vec![0u8; 1024])
    }
}

pub struct InGen {
    manifest_thread: Option<std::thread::JoinHandle<()>>,
    key: [u8; 32],
    nonce: [u8; 12],
    dummy_encrypted_data: Vec<u8>,
    verifying_key: VerifyingKey,
    signature: Signature,
}

impl InGen {
    pub fn new() -> Self {
        // Setup dummy keys and data for benchmarking the cost of crypto
        let bytes: [u8; 32] = rand::random();
        let signing_key: SigningKey = SigningKey::from_bytes(&bytes);
        let verifying_key = signing_key.verifying_key();
        let msg: &[u8] = b"valid_license_token";
        let signature = signing_key.sign(msg);

        let key_bytes: [u8; 32] = rand::random();
        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
        
        let nonce_bytes: [u8; 12] = rand::random();
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let cipher = Aes256Gcm::new(key);
        let dummy_data = vec![42u8; 1024 * 1024]; // 1MB asset
        let dummy_encrypted_data = cipher.encrypt(&nonce, dummy_data.as_ref()).unwrap();

        Self {
            manifest_thread: None,
            key: key_bytes,
            nonce: nonce_bytes,
            dummy_encrypted_data,
            verifying_key,
            signature,
        }
    }
}

impl ProtectionProfile for InGen {
    fn on_startup(&mut self) -> Result<(), &'static str> {
        let start_time = Instant::now();
        write_audit_log("AUTH", "Initiating Ed25519 License Signature Verification...");
        
        let msg: &[u8] = b"valid_license_token";
        if self.verifying_key.verify(msg, &self.signature).is_err() {
            write_audit_log("CRITICAL", "Auth Failed: Invalid License Signature! System lockdown initiated.");
            return Err("License verification failed");
        }
        
        write_audit_log("AUTH", &format!("License verified successfully. Latency: {:.2}ms", start_time.elapsed().as_secs_f64() * 1000.0));

        // 2. Async manifest integrity check (spawns thread)
        let thread_handle = std::thread::spawn(move || {
            let dummy_manifest = vec![0u8; 1024 * 1024]; // 1MB manifest
            let mut check_count = 0;
            loop {
                std::thread::sleep(Duration::from_millis(3000));
                check_count += 1;
                let watch_start = Instant::now();
                
                // Anti-Tamper Check
                if TAMPER_FLAG.load(Ordering::Relaxed) {
                    write_audit_log("CRITICAL", "Memory Tamper Detected via external hook! Hash mismatch.");
                    break;
                }
                
                // Simulate computing the hash
                let mut hasher = Sha256::new();
                hasher.update(&dummy_manifest);
                let _hash = hasher.finalize();
                
                write_audit_log("WATCHDOG", &format!("Integrity check #{} passed. Latency: {:.2}ms", check_count, watch_start.elapsed().as_secs_f64() * 1000.0));
            }
        });
        
        self.manifest_thread = Some(thread_handle);
        Ok(())
    }

    fn on_checkpoint(&mut self) -> Result<(), &'static str> {
        Ok(())
    }

    fn load_protected_asset(&mut self) -> Result<Vec<u8>, String> {
        let cipher = Aes256Gcm::new_from_slice(&self.key).unwrap();
        let nonce = Nonce::from_slice(&self.nonce);
        cipher.decrypt(nonce, self.dummy_encrypted_data.as_ref())
            .map_err(|_| "Decryption failed".into())
    }
}

pub struct HeavyReasonable {
    key: [u8; 32],
    nonce: [u8; 12],
    dummy_encrypted_data: Vec<u8>,
    verifying_key: VerifyingKey,
    signature: Signature,
    checkpoint_counter: usize,
    memory_block: Vec<u8>,
}

impl HeavyReasonable {
    pub fn new() -> Self {
        let bytes: [u8; 32] = rand::random();
        let signing_key: SigningKey = SigningKey::from_bytes(&bytes);
        let verifying_key = signing_key.verifying_key();
        let msg: &[u8] = b"valid_license_token";
        let signature = signing_key.sign(msg);

        let key_bytes: [u8; 32] = rand::random();
        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
        
        let nonce_bytes: [u8; 12] = rand::random();
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let cipher = Aes256Gcm::new(key);
        let dummy_data = vec![42u8; 1024 * 1024];
        let dummy_encrypted_data = cipher.encrypt(&nonce, dummy_data.as_ref()).unwrap();

        Self {
            key: key_bytes,
            nonce: nonce_bytes,
            dummy_encrypted_data,
            verifying_key,
            signature,
            checkpoint_counter: 0,
            memory_block: vec![1u8; 1024 * 1024 * 2], // 2MB block to hash synchronously
        }
    }
}

impl ProtectionProfile for HeavyReasonable {
    fn on_startup(&mut self) -> Result<(), &'static str> {
        let msg: &[u8] = b"valid_license_token";
        self.verifying_key.verify(msg, &self.signature).map_err(|_| "Verification failed")?;
        Ok(())
    }

    fn on_checkpoint(&mut self) -> Result<(), &'static str> {
        self.checkpoint_counter += 1;
        // Moderate sync point that creates frametime pressure.
        // E.g., every 60 frames, do a synchronous hash of a memory region
        if self.checkpoint_counter % 60 == 0 {
            let mut hasher = Sha256::new();
            hasher.update(&self.memory_block);
            let result = hasher.finalize();
            self.memory_block[0] ^= result[0]; // Avoid optimization
        }
        
        // Artificial memory traffic to simulate VM thunk overhead on every checkpoint
        for i in 0..1000 {
            let idx = (self.checkpoint_counter * i) % self.memory_block.len();
            self.memory_block[idx] = self.memory_block[idx].wrapping_add(1);
        }
        Ok(())
    }

    fn load_protected_asset(&mut self) -> Result<Vec<u8>, String> {
        let cipher = Aes256Gcm::new_from_slice(&self.key).unwrap();
        let nonce = Nonce::from_slice(&self.nonce);
        cipher.decrypt(nonce, self.dummy_encrypted_data.as_ref())
            .map_err(|_| "Decryption failed".into())
    }
}

pub struct HeavyAbusive {
    // Similar to HeavyReasonable but much more frequent/larger
    checkpoint_counter: usize,
    memory_block: Vec<u8>,
}

impl HeavyAbusive {
    pub fn new() -> Self {
        Self {
            checkpoint_counter: 0,
            memory_block: vec![1u8; 1024 * 1024 * 5], // 5MB block
        }
    }
}

impl ProtectionProfile for HeavyAbusive {
    fn on_startup(&mut self) -> Result<(), &'static str> { Ok(()) }

    fn on_checkpoint(&mut self) -> Result<(), &'static str> {
        self.checkpoint_counter += 1;
        // Sync hash every 5 frames instead of 60
        if self.checkpoint_counter % 5 == 0 {
            let mut hasher = Sha256::new();
            hasher.update(&self.memory_block);
            let _ = hasher.finalize();
        }
        
        // Huge artificial memory traffic
        for i in 0..50000 {
            let idx = (self.checkpoint_counter * i) % self.memory_block.len();
            self.memory_block[idx] = self.memory_block[idx].wrapping_add(1);
        }
        Ok(())
    }

    fn load_protected_asset(&mut self) -> Result<Vec<u8>, String> {
        Ok(vec![])
    }
}

// -----------------------------------------------------------------------
// AlwaysOnline: Simulates continuous synchronous heartbeat checks
// -----------------------------------------------------------------------
impl AlwaysOnline {
    pub fn new() -> Self { Self }
}

impl ProtectionProfile for AlwaysOnline {
    fn on_startup(&mut self) -> Result<(), &'static str> {
        std::thread::sleep(Duration::from_millis(50)); // Simulating auth handshake
        Ok(())
    }

    fn on_checkpoint(&mut self) -> Result<(), &'static str> {
        // 5% chance every frame to simulate waiting for a network heartbeat ack
        if rand::random::<f32>() < 0.05 {
            std::thread::sleep(Duration::from_millis(15)); // 15ms stutter
        }
        Ok(())
    }

    fn load_protected_asset(&mut self) -> Result<Vec<u8>, String> {
        Ok(vec![0; 1024])
    }
}

// -----------------------------------------------------------------------
// VMAntitamper: Simulates heavy instruction virtualization/obfuscation
// -----------------------------------------------------------------------
impl VMAntitamper {
    pub fn new() -> Self { Self }
}

impl ProtectionProfile for VMAntitamper {
    fn on_startup(&mut self) -> Result<(), &'static str> {
        Ok(())
    }

    fn on_checkpoint(&mut self) -> Result<(), &'static str> {
        // Simulates heavy instruction virtualization logic (sync busy-wait / hashing)
        // Causes a massive 3-5ms delay on EVERY frame reliably (like Denuvo triggers)
        let mut data = vec![0u8; 1024 * 512]; // 512 KB
        for i in 0..data.len() {
            data[i] = data[i].wrapping_add(1);
        }
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let _ = hasher.finalize();
        Ok(())
    }

    fn load_protected_asset(&mut self) -> Result<Vec<u8>, String> {
        Ok(vec![0; 1024])
    }
}
