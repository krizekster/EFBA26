use clap::Parser;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce
};
use ed25519_dalek::{Signer, SigningKey};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    input: PathBuf,

    #[arg(short, long)]
    protect: PathBuf,

    #[arg(short, long)]
    key: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
struct Manifest {
    file: String,
    sha256: String,
    nonce: String,
    tier: String,
}

fn main() {
    let args = Args::parse();
    println!("Sealing asset: {:?}", args.input);

    let data = fs::read(&args.input).expect("Failed to read input file");
    
    let key_bytes: [u8; 32] = rand::random();
    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
    
    let nonce_bytes: [u8; 12] = rand::random();
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let cipher = Aes256Gcm::new(key);
    let ciphertext = cipher.encrypt(nonce, data.as_ref()).expect("Encryption failed");
    
    let mut protected_path = args.input.clone();
    protected_path.set_extension("enc");
    fs::write(&protected_path, &ciphertext).expect("Failed to write protected file");
    
    let mut hasher = Sha256::new();
    hasher.update(&ciphertext);
    let hash = hasher.finalize();
    
    let hash_hex = hash.iter().map(|b| format!("{:02x}", b)).collect::<String>();
    let nonce_hex = nonce_bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>();
    
    let manifest = Manifest {
        file: protected_path.file_name().unwrap().to_string_lossy().into_owned(),
        sha256: hash_hex,
        nonce: nonce_hex,
        tier: "premium".to_string(),
    };
    
    let manifest_json = serde_json::to_string_pretty(&manifest).unwrap();
    fs::write(&args.protect, &manifest_json).expect("Failed to write manifest");
    
    let sk_bytes: [u8; 32] = rand::random();
    let signing_key: SigningKey = SigningKey::from_bytes(&sk_bytes);
    let signature = signing_key.sign(manifest_json.as_bytes());
    
    let pub_key = signing_key.verifying_key();
    fs::write(&args.key, pub_key.as_bytes()).expect("Failed to write public key");
    
    fs::write("symmetric.key", key_bytes).expect("Failed to write symmetric key");
    fs::write("manifest.sig", signature.to_bytes()).expect("Failed to write signature");
    
    println!("Sealing complete.");
    println!("Protected asset written to {:?}", protected_path);
    println!("Manifest written to {:?}", args.protect);
    println!("Public key written to {:?}", args.key);
}
