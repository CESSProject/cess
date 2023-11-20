#![cfg_attr(not(feature = "std"), no_std)]

use rsa::{pkcs1::DecodeRsaPublicKey, Pkcs1v15Sign, PublicKey};

pub fn verify_rsa(key: &[u8], msg: &[u8], sig: &[u8]) -> bool {
	let pk = rsa::RsaPublicKey::from_pkcs1_der(key).unwrap();

	match pk.verify(Pkcs1v15Sign::new_raw(), msg, sig) {
		Ok(()) => return true,
		Err(_) => return false,
	};
}

#[cfg(test)]
mod test {
	use rsa::{
		pkcs1::{DecodeRsaPublicKey, EncodeRsaPublicKey},
		pkcs8::DecodePrivateKey,
		Pkcs1v15Sign, PublicKey, RsaPrivateKey, RsaPublicKey,
	};

	#[test]
	fn cryptos_rsa() {
		let priv_key = RsaPrivateKey::from_pkcs8_der(&hex::decode("308204bd020100300d06092a864886f70d0101010500048204a7308204a3020100028201010097f726d8bc510040ef6552b5861e54ab781527c4d852110ae14e5b23b6083201a4ebcec9e9dfae7d8a33460e14c634c7098e41b7fb861672f2cda91c73d55252aa3f39183b88a2e77beabbaff45fe8fe82112ea3c95df4f0b31e2bdd43e2a11635a14ddd7c30153eb56c3f0a07a05caa976f337000b7a8db4e691d2f54229f1176c27304d06e384c16cac868d26166b7563f6c32b315ec93398e4c43f3eba2ca93a394ed07e57afa4bf963f29d2288af4a8c499109833858f9af111a5d480f3255e65b7ceb1a68c8a1a089f903310777cf647f6cd7fbd6e4b4dddf0ca342d6b96454410647f02df76bc1ee44435ca1df012b9689daddd5e8240579203b814b050203010001028201000cc6c4c7581ddf4d9653087f26858a4cd84dbf0837bfbe9b11924aeb57b49cafb2b3f8b0d52eef36b2d5d8bffa5327c0cc36dfa39e4c09bb245ad22b083a192fc60c86ba58d7060b3c49e1f9cb2bfd24d8bea513342ce8190c962ecded953241f1c45c0d911161d7e1dcf5f7dbe849a236152d57ef5781a4de94cbd55cd784540c633f4ce96a053d6a19533b9830c7b8396f190f020099a4e481e24a41022a1191330ac599a7199ca776913db12be96cf5b949ad5d73be9d3b66311a8ba10a48b00a9deedbccc015aace97fb79ae19bbe39fa0103772d4a25bf7e4f35c2ede2943a935423720d4f3059994f171650558bc1bfabf725d10916095c162b8717cc902818100c393e0b47b91321fef951ad8fb1cdc1ec928e73f715f673ff093847d380dcff6c040691ac7cdcac45bf0ed38299a10b1349a56b60178de81e38095251dda7b2bb0ca96c5457fe162daf9b7a0bf30d1a8eae47340d75622dd72c22aa12f3fb45154896b791ab4a5275be06657098bb9def81010297981b0d531af2c4ee08da78b02818100c6ea027695077584eca26d7ef4b4e361f3cc583ff5fff2114daebbb0a6a92dff11861c7596626e86bad32fd11ae7683f8ad45599eefe2cd2117825e0c5590be86e2187e0bb1636a104cde7b6b0a573e169f56c2ae8ff171eece31295964a3426009fc4c2672ace0be0d59c99c427c0fbf8e7e9d4636165e9ba8803c24f53a9af02818100b24e0e8ddd1e09c9cdde6d64a6c3aff72d446a578fdfffbcee733f55fe15b1a4efaf89634e07d3b5e370aa850a8098794650f37ee9a6ad8d53c175b82a187734e4f03e36c9df05b7df95cd10f35de9b78bb70d506f41eb75635b9c0be98cb5b37453f8b4a7614c34aef1cdbbca4b26011ebd5e4ec1a5387795dd7392d1ecb37302818045146fd68edb104d21812755b7d63a418251ad344952a1d6b08bc6530b0e2613371ac437720aad27cd2a1aa91c16d1757fd94e012fa6c61a0e4713a083e8f0e1bf9d957ace7e606a7b28a7182330d295ae1eb57a1180c59ecfd5ec5656e35e48f45e880e9b959a093603f966cd60a0fce0ec69a081030a49a9a622e8107495b10281806de9944afeb9670b92eb33b3afcd4890c20b3ba9f7d055305d5027b2ccd5d8f565488b3f56342f04968d29cdaee716ad333868beecbd7df5cd2aeeba0c1c4d810f78162e0a02e3c1aa54ee9103469efc6cf3542e9292bddffc8d328996a8bb67f78a7701e9f113ad790bdfd981d39cf116b5ee41dd5e42fcc2b1f3d11e6a7d15").unwrap()).unwrap();
		// let skey = RsaPrivateKey::new(&mut rng, bits).expect("failed to generate a key");
		let pub_key = RsaPublicKey::from(&priv_key);
		println!("{:?}", hex::encode(pub_key.to_pkcs1_der().unwrap().to_vec()));

		let original_text = [
			193, 246, 62, 231, 50, 88, 107, 103, 221, 158, 61, 148, 6, 248, 29, 248, 56, 172, 222,
			109, 191, 170, 225, 72, 211, 29, 42, 18, 51, 234, 205, 136,
		];

		// let priv_key_der = priv_key.to_pkcs1_der().unwrap();
		// println!("priv_key_der: {:?}", priv_key_der.as_bytes());
		// println!();

		let doc = pub_key.to_pkcs1_der().unwrap();
		// println!("pub_key: {:?}, length: {}", doc.as_bytes(), doc.as_bytes().len());

		let pk = rsa::RsaPublicKey::from_pkcs1_der(doc.as_bytes()).unwrap();

		let binding = priv_key.sign(Pkcs1v15Sign::new_raw(), &original_text).unwrap();
		let sig: &[u8] = binding.as_ref();
		println!("sig is: {:?}, sig is length: {:?}", sig, sig.len());

		let result = pk.verify(Pkcs1v15Sign::new_raw(), &original_text, &sig);

		println!("result: {:?}", result);
	}
}
