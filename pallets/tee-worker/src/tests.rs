
// use super::*;
// use libp2p::core::{
// 	multiaddr::{Multiaddr, Protocol},
// 	PeerId,
// };

// #[test]
// fn cryptos_are_compatible() {
// 	use sp_core::crypto::Pair;

// 	let libp2p_secret = libp2p::identity::Keypair::generate_ed25519();
// 	let libp2p_public = libp2p_secret.public();

// 	let sp_core_secret = {
// 		let libp2p::identity::Keypair::Ed25519(libp2p_ed_secret) = libp2p_secret.clone();
// 		sp_core::ed25519::Pair::from_seed_slice(&libp2p_ed_secret.secret().as_ref()).unwrap()
// 	};

// 	let sp_core_public = sp_core_secret.public();

// 	println!("libp2p_public: {:?}, sp_core public: {:?}", libp2p_public, sp_core_public);

// 	let message = b"we are more powerful than not to be better";

// 	let libp2p_signature = libp2p_secret.sign(message).unwrap();
// 	let sp_core_signature = sp_core_secret.sign(message); // no error expected...

// 	assert!(sp_core::ed25519::Pair::verify(
// 		&sp_core::ed25519::Signature::from_slice(&libp2p_signature).unwrap(),
// 		message,
// 		&sp_core_public
// 	));
// 	assert!(libp2p_public.verify(message, sp_core_signature.as_ref()));
// }
