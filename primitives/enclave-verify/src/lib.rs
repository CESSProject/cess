#![cfg_attr(not(feature = "std"), no_std)]
// use frame_support::ensure;
use sp_std::vec::Vec;
// use sp_application_crypto::{
// 	ecdsa::{Signature, Public},
// };
// use sp_io::hashing::sha2_256;
// use serde_json::Value;
use cp_cess_common::*;
// #[cfg(feature = "std")]
// use sp_externalities::{Externalities, ExternalitiesExt};
// use sp_runtime_interface::{
// 	runtime_interface,
// };
use rsa::{
    PublicKey,
    Pkcs1v15Sign, 
    pkcs1::DecodeRsaPublicKey,
};
use ic_verify_bls_signature::{
    Signature as BLSSignature,
    PublicKey as BLSPubilc,
};
use sp_std::prelude::ToOwned;
use serde_json::Value;
use scale_info::prelude::string::String;
#[cfg(test)]

#[cfg(test)]
use rsa::{
    RsaPublicKey, RsaPrivateKey, 
    pkcs1::{/*EncodeRsaPrivateKey, DecodeRsaPrivateKey, */EncodeRsaPublicKey},
    pkcs8::DecodePrivateKey,
};

// #[cfg(feature = "std")]
// sp_externalities::decl_extension! {
// 	pub struct UseDalekExt;
// }
// #[cfg(feature = "std")]
// impl Default for UseDalekExt {
// 	fn default() -> Self {
// 		Self
// 	}
// }

pub static IAS_SERVER_ROOTS: webpki::TLSServerTrustAnchors = webpki::TLSServerTrustAnchors(&[
    /*
     * -----BEGIN CERTIFICATE-----
     * MIIFSzCCA7OgAwIBAgIJANEHdl0yo7CUMA0GCSqGSIb3DQEBCwUAMH4xCzAJBgNV
     * BAYTAlVTMQswCQYDVQQIDAJDQTEUMBIGA1UEBwwLU2FudGEgQ2xhcmExGjAYBgNV
     * BAoMEUludGVsIENvcnBvcmF0aW9uMTAwLgYDVQQDDCdJbnRlbCBTR1ggQXR0ZXN0
     * YXRpb24gUmVwb3J0IFNpZ25pbmcgQ0EwIBcNMTYxMTE0MTUzNzMxWhgPMjA0OTEy
     * MzEyMzU5NTlaMH4xCzAJBgNVBAYTAlVTMQswCQYDVQQIDAJDQTEUMBIGA1UEBwwL
     * U2FudGEgQ2xhcmExGjAYBgNVBAoMEUludGVsIENvcnBvcmF0aW9uMTAwLgYDVQQD
     * DCdJbnRlbCBTR1ggQXR0ZXN0YXRpb24gUmVwb3J0IFNpZ25pbmcgQ0EwggGiMA0G
     * CSqGSIb3DQEBAQUAA4IBjwAwggGKAoIBgQCfPGR+tXc8u1EtJzLA10Feu1Wg+p7e
     * LmSRmeaCHbkQ1TF3Nwl3RmpqXkeGzNLd69QUnWovYyVSndEMyYc3sHecGgfinEeh
     * rgBJSEdsSJ9FpaFdesjsxqzGRa20PYdnnfWcCTvFoulpbFR4VBuXnnVLVzkUvlXT
     * L/TAnd8nIZk0zZkFJ7P5LtePvykkar7LcSQO85wtcQe0R1Raf/sQ6wYKaKmFgCGe
     * NpEJUmg4ktal4qgIAxk+QHUxQE42sxViN5mqglB0QJdUot/o9a/V/mMeH8KvOAiQ
     * byinkNndn+Bgk5sSV5DFgF0DffVqmVMblt5p3jPtImzBIH0QQrXJq39AT8cRwP5H
     * afuVeLHcDsRp6hol4P+ZFIhu8mmbI1u0hH3W/0C2BuYXB5PC+5izFFh/nP0lc2Lf
     * 6rELO9LZdnOhpL1ExFOq9H/B8tPQ84T3Sgb4nAifDabNt/zu6MmCGo5U8lwEFtGM
     * RoOaX4AS+909x00lYnmtwsDVWv9vBiJCXRsCAwEAAaOByTCBxjBgBgNVHR8EWTBX
     * MFWgU6BRhk9odHRwOi8vdHJ1c3RlZHNlcnZpY2VzLmludGVsLmNvbS9jb250ZW50
     * L0NSTC9TR1gvQXR0ZXN0YXRpb25SZXBvcnRTaWduaW5nQ0EuY3JsMB0GA1UdDgQW
     * BBR4Q3t2pn680K9+QjfrNXw7hwFRPDAfBgNVHSMEGDAWgBR4Q3t2pn680K9+Qjfr
     * NXw7hwFRPDAOBgNVHQ8BAf8EBAMCAQYwEgYDVR0TAQH/BAgwBgEB/wIBADANBgkq
     * hkiG9w0BAQsFAAOCAYEAeF8tYMXICvQqeXYQITkV2oLJsp6J4JAqJabHWxYJHGir
     * IEqucRiJSSx+HjIJEUVaj8E0QjEud6Y5lNmXlcjqRXaCPOqK0eGRz6hi+ripMtPZ
     * sFNaBwLQVV905SDjAzDzNIDnrcnXyB4gcDFCvwDFKKgLRjOB/WAqgscDUoGq5ZVi
     * zLUzTqiQPmULAQaB9c6Oti6snEFJiCQ67JLyW/E83/frzCmO5Ru6WjU4tmsmy8Ra
     * Ud4APK0wZTGtfPXU7w+IBdG5Ez0kE1qzxGQaL4gINJ1zMyleDnbuS8UicjJijvqA
     * 152Sq049ESDz+1rRGc2NVEqh1KaGXmtXvqxXcTB+Ljy5Bw2ke0v8iGngFBPqCTVB
     * 3op5KBG3RjbF6RRSzwzuWfL7QErNC8WEy5yDVARzTA5+xmBc388v9Dm21HGfcC8O
     * DD+gT9sSpssq0ascmvH49MOgjt1yoysLtdCtJW/9FZpoOypaHx0R+mJTLwPXVMrv
     * DaVzWh5aiEx+idkSGMnX
     * -----END CERTIFICATE-----
     */
    webpki::TrustAnchor {
        subject: b"1\x0b0\t\x06\x03U\x04\x06\x13\x02US1\x0b0\t\x06\x03U\x04\x08\x0c\x02CA1\x140\x12\x06\x03U\x04\x07\x0c\x0bSanta Clara1\x1a0\x18\x06\x03U\x04\n\x0c\x11Intel Corporation100.\x06\x03U\x04\x03\x0c\'Intel SGX Attestation Report Signing CA",
        spki: b"0\r\x06\t*\x86H\x86\xf7\r\x01\x01\x01\x05\x00\x03\x82\x01\x8f\x000\x82\x01\x8a\x02\x82\x01\x81\x00\x9f<d~\xb5w<\xbbQ-\'2\xc0\xd7A^\xbbU\xa0\xfa\x9e\xde.d\x91\x99\xe6\x82\x1d\xb9\x10\xd51w7\twFjj^G\x86\xcc\xd2\xdd\xeb\xd4\x14\x9dj/c%R\x9d\xd1\x0c\xc9\x877\xb0w\x9c\x1a\x07\xe2\x9cG\xa1\xae\x00IHGlH\x9fE\xa5\xa1]z\xc8\xec\xc6\xac\xc6E\xad\xb4=\x87g\x9d\xf5\x9c\t;\xc5\xa2\xe9ilTxT\x1b\x97\x9euKW9\x14\xbeU\xd3/\xf4\xc0\x9d\xdf\'!\x994\xcd\x99\x05\'\xb3\xf9.\xd7\x8f\xbf)$j\xbe\xcbq$\x0e\xf3\x9c-q\x07\xb4GTZ\x7f\xfb\x10\xeb\x06\nh\xa9\x85\x80!\x9e6\x91\tRh8\x92\xd6\xa5\xe2\xa8\x08\x03\x19>@u1@N6\xb3\x15b7\x99\xaa\x82Pt@\x97T\xa2\xdf\xe8\xf5\xaf\xd5\xfec\x1e\x1f\xc2\xaf8\x08\x90o(\xa7\x90\xd9\xdd\x9f\xe0`\x93\x9b\x12W\x90\xc5\x80]\x03}\xf5j\x99S\x1b\x96\xdei\xde3\xed\"l\xc1 }\x10B\xb5\xc9\xab\x7f@O\xc7\x11\xc0\xfeGi\xfb\x95x\xb1\xdc\x0e\xc4i\xea\x1a%\xe0\xff\x99\x14\x88n\xf2i\x9b#[\xb4\x84}\xd6\xff@\xb6\x06\xe6\x17\x07\x93\xc2\xfb\x98\xb3\x14X\x7f\x9c\xfd%sb\xdf\xea\xb1\x0b;\xd2\xd9vs\xa1\xa4\xbdD\xc4S\xaa\xf4\x7f\xc1\xf2\xd3\xd0\xf3\x84\xf7J\x06\xf8\x9c\x08\x9f\r\xa6\xcd\xb7\xfc\xee\xe8\xc9\x82\x1a\x8eT\xf2\\\x04\x16\xd1\x8cF\x83\x9a_\x80\x12\xfb\xdd=\xc7M%by\xad\xc2\xc0\xd5Z\xffo\x06\"B]\x1b\x02\x03\x01\x00\x01",
        name_constraints: None
    },
]);

type SignatureAlgorithms = &'static [&'static webpki::SignatureAlgorithm];
static SUPPORTED_SIG_ALGS: SignatureAlgorithms = &[
    &webpki::RSA_PKCS1_2048_8192_SHA256,
    &webpki::RSA_PKCS1_2048_8192_SHA384,
    &webpki::RSA_PKCS1_2048_8192_SHA512,
    &webpki::RSA_PKCS1_3072_8192_SHA384,
];

// pub fn u8v_to_hexstr(x: &[u8]) -> String {
//     // produce a hexnum string from a byte vector
//     let mut s = String::new();
//     for ix in 0..x.len() {
//         s.push_str(&format!("{:02x}", x[ix]));
//     }
//     s
// }

pub fn hexstr_to_u8v(s: &str, x: &mut [u8]) {
    let nx = x.len();
    let mut pos = 0;
    let mut val: u8 = 0;
    let mut cct = 0;
    for c in s.chars() {
        if pos < nx {
            match c.to_digit(16) {
                Some(d) => {
                    val += d as u8;
                    cct += 1;
                    if (cct & 1) == 0 {
                        x[pos] = val;
                        pos += 1;
                        val = 0;
                    } else {
                        val <<= 4;
                    }
                }
                None => panic!("Invalid hex digit"),
            }
        } else {
            break;
        }
    }
    for ix in pos..nx {
        x[ix] = val;
        val = 0;
    }
}

pub fn verify_miner_cert(
    ias_sig: &ReportSign,
    ias_cert: &Cert,
    report_json_raw: &Report,
    id_hashing: &[u8],
) -> Option<u8> {
    let ias_cert_dec = match base64::decode_config(ias_cert, base64::STANDARD) {
        Ok(c) => c,
        Err(_) => return Option::None,
    };
    let sig_cert: webpki::EndEntityCert = match webpki::EndEntityCert::from(ias_cert_dec.as_slice()) {
        Ok(c) => c,
        Err(_) => return Option::None,
    };

    let intermediate_report: Vec<&[u8]> = Vec::new();
    //2022-12-09 00:00:00
    let now_func = webpki::Time::from_seconds_since_unix_epoch(1670515200); 

    if let Err(_e) = sig_cert.verify_is_valid_tls_server_cert(
        SUPPORTED_SIG_ALGS,
        &IAS_SERVER_ROOTS,
        &intermediate_report,
        now_func
    ) {return Option::None;}

    let ias_sig_dec: Vec<u8> = match base64::decode(ias_sig) {
        Ok(value) => value,
        Err(_) => return Option::None,
    };

    if let Err(_e) = sig_cert.verify_signature(
        &webpki::RSA_PKCS1_2048_8192_SHA256,
        &report_json_raw,
        &ias_sig_dec,
    ) {return Option::None;}

    let some_quote_body: Value = match serde_json::from_slice(&report_json_raw) {
        Ok(body) => body,
        Err(_) => return Option::None,
    };

    if let Value::String(maybe_isv_quote_body) = &some_quote_body["isvEnclaveQuoteBody"] {
        let decoded_quote_body = match base64::decode(&maybe_isv_quote_body) {
            Ok(decoded_qb) => decoded_qb,
            Err(_) => return Option::None,
        };

        let quote_report: QuoteReport = QuoteReport::try_from(decoded_quote_body.as_slice()).unwrap();
        let slice = &quote_report.report_body.report_data[0..64];
        let id_hashing_hex = hex::encode(id_hashing);
        if &id_hashing_hex != slice {
            return Option::None;
        }
    }


    // let some_quote_body: Value = match serde_json::from_slice(report_json_raw) {
    //     Ok(body) => body,
    //     Err(_) => return Option::None,
    // };

    // if let Value::String(maybe_isv_quote_body) = &some_quote_body["isvEnclaveQuoteBody"] {
    //     let decoded_quote_body = match base64::decode(&maybe_isv_quote_body) {
    //         Ok(decoded_qb) => decoded_qb,
    //         Err(_) => return Option::None,
    //     };

    //     let id_code: [u8; 32] =  match decoded_quote_body[112..144].try_into() {
    //         Ok(code) => code,
    //         Err(_) => return Option::None,
    //     };

    //     if !mrenclave_codes.contains(&id_code) {
    //         return Option::None;
    //     }

    //     let quote_mr_signer: [u8; 32] =  match decoded_quote_body[176..208].try_into() {
    //         Ok(mr_signer) => mr_signer,
    //         Err(_) => return Option::None,
    //     };

    //     if mr_signer != &quote_mr_signer {
    //         return Option::None;
    //     }

    //     let quote_pk: [u8; 33] = match decoded_quote_body[368..401].try_into() {
    //         Ok(pk) => pk,
    //         Err(_) => return Option::None,
    //     };

    //     let pk = Public::from_raw(quote_pk);

    //     let data: Vec<u8> = [&quote_body[..], &ias_sig[..], &ias_cert[..]].concat();
    //     let result = sp_io::crypto::ecdsa_verify_prehashed(quote_sig, &sha2_256(&data), &pk);

    //     if !result {
    //         return Option::None;
    //     }

    //     return Option::Some(pk);
    // };

    Option::Some(1)
}

pub fn verify_rsa(key: &[u8], msg: &[u8], sig: &[u8]) -> bool {
    let pk = rsa::RsaPublicKey::from_pkcs1_der(key).unwrap();

    match pk.verify(Pkcs1v15Sign::new_raw(), msg, sig) {
        Ok(()) => return true,
        Err(_) => return false,
    };
}

pub fn verify_bls(key: &[u8], msg: &[u8], sig: &[u8]) -> Result<(), ()> {
    let puk = BLSPubilc::deserialize(key).unwrap();
    log::info!("bls puk: {:?}", puk);
    let sig = BLSSignature::deserialize(sig).unwrap();
    puk.verify(&msg, &sig)
}

// pub fn sig_rsa(key: &[u8], msg: &[u8]) -> &[u8] {`

// }

#[derive(PartialEq, Eq, Encode, Decode, Clone)]
pub struct PoISKey {
	g: [u8; 256],
	n: [u8; 256],
}


use codec::{Decode, Encode};
#[test]
fn cryptos_rsa() {
	let priv_key = RsaPrivateKey::from_pkcs8_der(&hex::decode("308204bd020100300d06092a864886f70d0101010500048204a7308204a3020100028201010097f726d8bc510040ef6552b5861e54ab781527c4d852110ae14e5b23b6083201a4ebcec9e9dfae7d8a33460e14c634c7098e41b7fb861672f2cda91c73d55252aa3f39183b88a2e77beabbaff45fe8fe82112ea3c95df4f0b31e2bdd43e2a11635a14ddd7c30153eb56c3f0a07a05caa976f337000b7a8db4e691d2f54229f1176c27304d06e384c16cac868d26166b7563f6c32b315ec93398e4c43f3eba2ca93a394ed07e57afa4bf963f29d2288af4a8c499109833858f9af111a5d480f3255e65b7ceb1a68c8a1a089f903310777cf647f6cd7fbd6e4b4dddf0ca342d6b96454410647f02df76bc1ee44435ca1df012b9689daddd5e8240579203b814b050203010001028201000cc6c4c7581ddf4d9653087f26858a4cd84dbf0837bfbe9b11924aeb57b49cafb2b3f8b0d52eef36b2d5d8bffa5327c0cc36dfa39e4c09bb245ad22b083a192fc60c86ba58d7060b3c49e1f9cb2bfd24d8bea513342ce8190c962ecded953241f1c45c0d911161d7e1dcf5f7dbe849a236152d57ef5781a4de94cbd55cd784540c633f4ce96a053d6a19533b9830c7b8396f190f020099a4e481e24a41022a1191330ac599a7199ca776913db12be96cf5b949ad5d73be9d3b66311a8ba10a48b00a9deedbccc015aace97fb79ae19bbe39fa0103772d4a25bf7e4f35c2ede2943a935423720d4f3059994f171650558bc1bfabf725d10916095c162b8717cc902818100c393e0b47b91321fef951ad8fb1cdc1ec928e73f715f673ff093847d380dcff6c040691ac7cdcac45bf0ed38299a10b1349a56b60178de81e38095251dda7b2bb0ca96c5457fe162daf9b7a0bf30d1a8eae47340d75622dd72c22aa12f3fb45154896b791ab4a5275be06657098bb9def81010297981b0d531af2c4ee08da78b02818100c6ea027695077584eca26d7ef4b4e361f3cc583ff5fff2114daebbb0a6a92dff11861c7596626e86bad32fd11ae7683f8ad45599eefe2cd2117825e0c5590be86e2187e0bb1636a104cde7b6b0a573e169f56c2ae8ff171eece31295964a3426009fc4c2672ace0be0d59c99c427c0fbf8e7e9d4636165e9ba8803c24f53a9af02818100b24e0e8ddd1e09c9cdde6d64a6c3aff72d446a578fdfffbcee733f55fe15b1a4efaf89634e07d3b5e370aa850a8098794650f37ee9a6ad8d53c175b82a187734e4f03e36c9df05b7df95cd10f35de9b78bb70d506f41eb75635b9c0be98cb5b37453f8b4a7614c34aef1cdbbca4b26011ebd5e4ec1a5387795dd7392d1ecb37302818045146fd68edb104d21812755b7d63a418251ad344952a1d6b08bc6530b0e2613371ac437720aad27cd2a1aa91c16d1757fd94e012fa6c61a0e4713a083e8f0e1bf9d957ace7e606a7b28a7182330d295ae1eb57a1180c59ecfd5ec5656e35e48f45e880e9b959a093603f966cd60a0fce0ec69a081030a49a9a622e8107495b10281806de9944afeb9670b92eb33b3afcd4890c20b3ba9f7d055305d5027b2ccd5d8f565488b3f56342f04968d29cdaee716ad333868beecbd7df5cd2aeeba0c1c4d810f78162e0a02e3c1aa54ee9103469efc6cf3542e9292bddffc8d328996a8bb67f78a7701e9f113ad790bdfd981d39cf116b5ee41dd5e42fcc2b1f3d11e6a7d15").unwrap()).unwrap();
	// let skey = RsaPrivateKey::new(&mut rng, bits).expect("failed to generate a key");
	let pub_key = RsaPublicKey::from(&priv_key);
	println!("{:?}",hex::encode(pub_key.to_pkcs1_der().unwrap().to_vec()));

	let original_text = [193, 246, 62, 231, 50, 88, 107, 103, 221, 158, 61, 148, 6, 248, 29, 248, 56, 172, 222, 109, 191, 170, 225, 72, 211, 29, 42, 18, 51, 234, 205, 136];

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

#[derive(Debug)]
pub enum EnclaveError {
    InvalidQuoteDataSize,

}

#[derive(Debug)]
pub struct QuoteReport {
  pub version: u16,
  pub sign_type: u16,
  pub report_body: QuoteReportBody,
}

#[derive(Debug)]
pub struct QuoteReportBody {
  pub mr_enclave: String,
  pub mr_signer: String,
  pub report_data: String,
}

impl TryFrom<&[u8]> for QuoteReport {
    type Error = EnclaveError;
  
    fn try_from(data: &[u8]) -> Result<QuoteReport, Self::Error> {
      if data.len() * 2 < 864 {
        return Err(EnclaveError::InvalidQuoteDataSize);
      }

      let hex_str = hex::encode(data);
      let report_body = QuoteReportBody {
        mr_enclave: (&hex_str[224..288]).to_owned(),
        mr_signer: (&hex_str[352..416]).to_owned(),
        report_data: (&hex_str[736..864]).to_owned(),
      };
      return Ok(QuoteReport { version: data[0] as u16, sign_type: data[2] as u16, report_body });
    }
}

// #[test]
// fn anaylize_cert() {
//     let report_json_raw: Report = [123, 34, 105, 100, 34, 58, 34, 49, 52, 53, 48, 49, 48, 48, 56, 52, 50, 57, 55, 53, 48, 51, 56, 53, 56, 57, 54, 54, 52, 56, 56, 57, 56, 49, 53, 53, 52, 52, 51, 53, 56, 48, 48, 55, 53, 56, 34, 44, 34, 116, 105, 109, 101, 115, 116, 97, 109, 112, 34, 58, 34, 50, 48, 50, 51, 45, 48, 56, 45, 50, 54, 84, 48, 50, 58, 48, 56, 58, 52, 51, 46, 53, 51, 52, 57, 53, 55, 34, 44, 34, 118, 101, 114, 115, 105, 111, 110, 34, 58, 52, 44, 34, 101, 112, 105, 100, 80, 115, 101, 117, 100, 111, 110, 121, 109, 34, 58, 34, 55, 43, 74, 112, 105, 53, 82, 83, 68, 117, 97, 54, 113, 49, 114, 109, 90, 72, 77, 112, 75, 100, 116, 80, 104, 102, 103, 111, 81, 106, 43, 55, 52, 85, 106, 111, 49, 49, 83, 67, 113, 118, 89, 107, 47, 85, 49, 47, 115, 48, 56, 72, 50, 109, 67, 101, 107, 110, 53, 119, 107, 82, 47, 53, 114, 71, 55, 66, 49, 122, 114, 104, 74, 49, 107, 121, 118, 101, 82, 47, 107, 72, 120, 98, 47, 71, 109, 75, 71, 51, 84, 99, 52, 83, 86, 74, 110, 78, 55, 75, 43, 47, 97, 48, 80, 110, 54, 121, 43, 106, 66, 75, 104, 106, 52, 79, 74, 101, 109, 102, 88, 75, 84, 104, 109, 49, 104, 89, 122, 52, 76, 65, 113, 53, 120, 69, 83, 82, 121, 68, 76, 89, 111, 71, 82, 89, 47, 57, 48, 118, 103, 97, 65, 89, 90, 109, 104, 53, 111, 85, 111, 48, 65, 98, 98, 111, 49, 99, 65, 102, 119, 61, 34, 44, 34, 97, 100, 118, 105, 115, 111, 114, 121, 85, 82, 76, 34, 58, 34, 104, 116, 116, 112, 115, 58, 47, 47, 115, 101, 99, 117, 114, 105, 116, 121, 45, 99, 101, 110, 116, 101, 114, 46, 105, 110, 116, 101, 108, 46, 99, 111, 109, 34, 44, 34, 97, 100, 118, 105, 115, 111, 114, 121, 73, 68, 115, 34, 58, 91, 34, 73, 78, 84, 69, 76, 45, 83, 65, 45, 48, 48, 51, 51, 52, 34, 44, 34, 73, 78, 84, 69, 76, 45, 83, 65, 45, 48, 48, 51, 56, 49, 34, 44, 34, 73, 78, 84, 69, 76, 45, 83, 65, 45, 48, 48, 51, 56, 57, 34, 44, 34, 73, 78, 84, 69, 76, 45, 83, 65, 45, 48, 48, 52, 55, 55, 34, 44, 34, 73, 78, 84, 69, 76, 45, 83, 65, 45, 48, 48, 54, 49, 52, 34, 44, 34, 73, 78, 84, 69, 76, 45, 83, 65, 45, 48, 48, 54, 49, 53, 34, 44, 34, 73, 78, 84, 69, 76, 45, 83, 65, 45, 48, 48, 54, 49, 55, 34, 44, 34, 73, 78, 84, 69, 76, 45, 83, 65, 45, 48, 48, 56, 50, 56, 34, 93, 44, 34, 105, 115, 118, 69, 110, 99, 108, 97, 118, 101, 81, 117, 111, 116, 101, 83, 116, 97, 116, 117, 115, 34, 58, 34, 71, 82, 79, 85, 80, 95, 79, 85, 84, 95, 79, 70, 95, 68, 65, 84, 69, 34, 44, 34, 112, 108, 97, 116, 102, 111, 114, 109, 73, 110, 102, 111, 66, 108, 111, 98, 34, 58, 34, 49, 53, 48, 50, 48, 48, 54, 53, 48, 52, 48, 48, 48, 49, 48, 48, 48, 48, 48, 70, 48, 70, 48, 50, 48, 50, 48, 49, 56, 48, 48, 69, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 68, 48, 48, 48, 48, 48, 67, 48, 48, 48, 48, 48, 48, 48, 50, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 66, 69, 52, 57, 65, 66, 51, 55, 55, 53, 56, 66, 70, 50, 67, 49, 65, 48, 70, 56, 52, 68, 66, 66, 56, 67, 67, 66, 48, 49, 52, 57, 67, 68, 50, 53, 69, 48, 51, 65, 56, 68, 66, 69, 48, 67, 50, 51, 70, 70, 66, 51, 49, 56, 66, 69, 70, 48, 68, 48, 66, 65, 49, 70, 53, 56, 51, 49, 52, 57, 55, 52, 49, 49, 67, 65, 55, 51, 70, 53, 56, 55, 49, 48, 65, 57, 52, 54, 66, 66, 70, 53, 66, 52, 56, 52, 66, 66, 66, 49, 67, 70, 69, 67, 54, 50, 49, 55, 67, 54, 55, 67, 70, 48, 56, 68, 68, 52, 69, 50, 70, 56, 67, 50, 56, 66, 69, 53, 52, 49, 66, 34, 44, 34, 105, 115, 118, 69, 110, 99, 108, 97, 118, 101, 81, 117, 111, 116, 101, 66, 111, 100, 121, 34, 58, 34, 65, 103, 65, 66, 65, 79, 81, 76, 65, 65, 65, 79, 65, 65, 52, 65, 65, 65, 65, 65, 65, 66, 54, 88, 104, 48, 69, 77, 82, 89, 43, 87, 101, 43, 48, 55, 65, 103, 99, 120, 111, 115, 85, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 67, 103, 55, 47, 66, 119, 101, 65, 66, 103, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 66, 119, 65, 65, 65, 65, 65, 65, 65, 65, 65, 72, 65, 65, 65, 65, 65, 65, 65, 65, 65, 75, 116, 102, 122, 78, 76, 100, 106, 54, 79, 100, 85, 99, 83, 87, 50, 77, 54, 66, 106, 83, 98, 116, 50, 79, 57, 100, 99, 122, 120, 103, 47, 111, 84, 106, 105, 51, 76, 84, 56, 87, 53, 69, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 67, 75, 116, 109, 66, 88, 114, 66, 121, 55, 49, 74, 43, 109, 82, 100, 107, 51, 65, 73, 57, 86, 68, 70, 118, 84, 86, 97, 56, 104, 50, 43, 112, 54, 99, 57, 87, 74, 51, 51, 79, 67, 90, 81, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 68, 68, 114, 56, 121, 89, 72, 57, 57, 79, 89, 52, 111, 68, 106, 111, 57, 80, 79, 111, 51, 103, 83, 47, 56, 75, 43, 114, 48, 56, 99, 86, 66, 110, 77, 107, 110, 88, 121, 89, 117, 43, 109, 119, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 65, 34, 125].to_vec().try_into().unwrap();
//     let ias_cert: Cert = [77, 73, 73, 69, 111, 84, 67, 67, 65, 119, 109, 103, 65, 119, 73, 66, 65, 103, 73, 74, 65, 78, 69, 72, 100, 108, 48, 121, 111, 55, 67, 87, 77, 65, 48, 71, 67, 83, 113, 71, 83, 73, 98, 51, 68, 81, 69, 66, 67, 119, 85, 65, 77, 72, 52, 120, 67, 122, 65, 74, 66, 103, 78, 86, 66, 65, 89, 84, 65, 108, 86, 84, 77, 81, 115, 119, 67, 81, 89, 68, 86, 81, 81, 73, 68, 65, 74, 68, 81, 84, 69, 85, 77, 66, 73, 71, 65, 49, 85, 69, 66, 119, 119, 76, 85, 50, 70, 117, 100, 71, 69, 103, 81, 50, 120, 104, 99, 109, 69, 120, 71, 106, 65, 89, 66, 103, 78, 86, 66, 65, 111, 77, 69, 85, 108, 117, 100, 71, 86, 115, 73, 69, 78, 118, 99, 110, 66, 118, 99, 109, 70, 48, 97, 87, 57, 117, 77, 84, 65, 119, 76, 103, 89, 68, 86, 81, 81, 68, 68, 67, 100, 74, 98, 110, 82, 108, 98, 67, 66, 84, 82, 49, 103, 103, 81, 88, 82, 48, 90, 88, 78, 48, 89, 88, 82, 112, 98, 50, 52, 103, 85, 109, 86, 119, 98, 51, 74, 48, 73, 70, 78, 112, 90, 50, 53, 112, 98, 109, 99, 103, 81, 48, 69, 119, 72, 104, 99, 78, 77, 84, 89, 120, 77, 84, 73, 121, 77, 68, 107, 122, 78, 106, 85, 52, 87, 104, 99, 78, 77, 106, 89, 120, 77, 84, 73, 119, 77, 68, 107, 122, 78, 106, 85, 52, 87, 106, 66, 55, 77, 81, 115, 119, 67, 81, 89, 68, 86, 81, 81, 71, 69, 119, 74, 86, 85, 122, 69, 76, 77, 65, 107, 71, 65, 49, 85, 69, 67, 65, 119, 67, 81, 48, 69, 120, 70, 68, 65, 83, 66, 103, 78, 86, 66, 65, 99, 77, 67, 49, 78, 104, 98, 110, 82, 104, 73, 69, 78, 115, 89, 88, 74, 104, 77, 82, 111, 119, 71, 65, 89, 68, 86, 81, 81, 75, 68, 66, 70, 74, 98, 110, 82, 108, 98, 67, 66, 68, 98, 51, 74, 119, 98, 51, 74, 104, 100, 71, 108, 118, 98, 106, 69, 116, 77, 67, 115, 71, 65, 49, 85, 69, 65, 119, 119, 107, 83, 87, 53, 48, 90, 87, 119, 103, 85, 48, 100, 89, 73, 69, 70, 48, 100, 71, 86, 122, 100, 71, 70, 48, 97, 87, 57, 117, 73, 70, 74, 108, 99, 71, 57, 121, 100, 67, 66, 84, 97, 87, 100, 117, 97, 87, 53, 110, 77, 73, 73, 66, 73, 106, 65, 78, 66, 103, 107, 113, 104, 107, 105, 71, 57, 119, 48, 66, 65, 81, 69, 70, 65, 65, 79, 67, 65, 81, 56, 65, 77, 73, 73, 66, 67, 103, 75, 67, 65, 81, 69, 65, 113, 88, 111, 116, 52, 79, 90, 117, 112, 104, 82, 56, 110, 117, 100, 70, 114, 65, 70, 105, 97, 71, 120, 120, 107, 103, 109, 97, 47, 69, 115, 47, 66, 65, 43, 116, 98, 101, 67, 84, 85, 82, 49, 48, 54, 65, 76, 49, 69, 78, 99, 87, 65, 52, 70, 88, 51, 75, 43, 69, 57, 66, 66, 76, 48, 47, 55, 88, 53, 114, 106, 53, 110, 73, 103, 88, 47, 82, 47, 49, 117, 98, 104, 107, 75, 87, 119, 57, 103, 102, 113, 80, 71, 51, 75, 101, 65, 116, 73, 100, 99, 118, 47, 117, 84, 79, 49, 121, 88, 118, 53, 48, 118, 113, 97, 80, 118, 69, 49, 67, 82, 67, 104, 118, 122, 100, 83, 47, 90, 69, 66, 113, 81, 53, 111, 86, 118, 76, 84, 80, 90, 51, 86, 69, 105, 99, 81, 106, 108, 121, 116, 75, 103, 78, 57, 99, 76, 110, 120, 98, 119, 116, 117, 118, 76, 85, 75, 55, 101, 121, 82, 80, 102, 74, 87, 47, 107, 115, 100, 100, 79, 122, 80, 56, 86, 66, 66, 110, 105, 111, 108, 89, 110, 82, 67, 68, 50, 106, 114, 77, 82, 90, 56, 110, 66, 77, 50, 90, 87, 89, 119, 110, 88, 110, 119, 89, 101, 79, 65, 72, 86, 43, 87, 57, 116, 79, 104, 65, 73, 109, 119, 82, 119, 75, 70, 47, 57, 53, 121, 65, 115, 86, 119, 100, 50, 49, 114, 121, 72, 77, 74, 66, 99, 71, 72, 55, 48, 113, 76, 97, 103, 90, 55, 84, 116, 121, 116, 43, 43, 113, 79, 47, 54, 43, 75, 65, 88, 74, 117, 75, 119, 90, 113, 106, 82, 108, 69, 116, 83, 69, 122, 56, 103, 90, 81, 101, 70, 102, 86, 89, 103, 99, 119, 83, 102, 111, 57, 54, 111, 83, 77, 65, 122, 86, 114, 55, 86, 48, 76, 54, 72, 83, 68, 76, 82, 110, 112, 98, 54, 120, 120, 109, 98, 80, 100, 113, 78, 111, 108, 52, 116, 81, 73, 68, 65, 81, 65, 66, 111, 52, 71, 107, 77, 73, 71, 104, 77, 66, 56, 71, 65, 49, 85, 100, 73, 119, 81, 89, 77, 66, 97, 65, 70, 72, 104, 68, 101, 51, 97, 109, 102, 114, 122, 81, 114, 51, 53, 67, 78, 43, 115, 49, 102, 68, 117, 72, 65, 86, 69, 56, 77, 65, 52, 71, 65, 49, 85, 100, 68, 119, 69, 66, 47, 119, 81, 69, 65, 119, 73, 71, 119, 68, 65, 77, 66, 103, 78, 86, 72, 82, 77, 66, 65, 102, 56, 69, 65, 106, 65, 65, 77, 71, 65, 71, 65, 49, 85, 100, 72, 119, 82, 90, 77, 70, 99, 119, 86, 97, 66, 84, 111, 70, 71, 71, 84, 50, 104, 48, 100, 72, 65, 54, 76, 121, 57, 48, 99, 110, 86, 122, 100, 71, 86, 107, 99, 50, 86, 121, 100, 109, 108, 106, 90, 88, 77, 117, 97, 87, 53, 48, 90, 87, 119, 117, 89, 50, 57, 116, 76, 50, 78, 118, 98, 110, 82, 108, 98, 110, 81, 118, 81, 49, 74, 77, 76, 49, 78, 72, 87, 67, 57, 66, 100, 72, 82, 108, 99, 51, 82, 104, 100, 71, 108, 118, 98, 108, 74, 108, 99, 71, 57, 121, 100, 70, 78, 112, 90, 50, 53, 112, 98, 109, 100, 68, 81, 83, 53, 106, 99, 109, 119, 119, 68, 81, 89, 74, 75, 111, 90, 73, 104, 118, 99, 78, 65, 81, 69, 76, 66, 81, 65, 68, 103, 103, 71, 66, 65, 71, 99, 73, 116, 104, 116, 99, 75, 57, 73, 86, 82, 122, 52, 114, 82, 113, 43, 90, 75, 69, 43, 55, 107, 53, 48, 47, 79, 120, 85, 115, 109, 87, 56, 97, 97, 118, 79, 122, 75, 98, 48, 105, 67, 120, 48, 55, 89, 81, 57, 114, 122, 105, 53, 110, 85, 55, 51, 116, 77, 69, 50, 121, 71, 82, 76, 122, 104, 83, 86, 105, 70, 115, 47, 76, 112, 70, 97, 57, 108, 112, 81, 76, 54, 74, 76, 49, 97, 81, 119, 109, 68, 82, 55, 52, 84, 120, 89, 71, 66, 65, 73, 105, 53, 102, 52, 73, 53, 84, 74, 111, 67, 67, 69, 113, 82, 72, 122, 57, 49, 107, 112, 71, 54, 85, 118, 121, 110, 50, 116, 76, 109, 110, 73, 100, 74, 98, 80, 69, 52, 118, 89, 118, 87, 76, 114, 116, 88, 88, 102, 70, 66, 83, 83, 80, 68, 52, 65, 102, 110, 55, 43, 51, 47, 88, 85, 103, 103, 65, 108, 99, 55, 111, 67, 84, 105, 122, 79, 102, 98, 98, 116, 79, 70, 108, 89, 65, 52, 103, 53, 75, 99, 89, 103, 83, 49, 74, 50, 90, 65, 101, 77, 81, 113, 98, 85, 100, 90, 115, 101, 90, 67, 99, 97, 90, 90, 90, 110, 54, 53, 116, 100, 113, 101, 101, 56, 85, 88, 90, 108, 68, 118, 120, 48, 43, 78, 100, 79, 48, 76, 82, 43, 53, 112, 70, 121, 43, 106, 117, 77, 48, 119, 87, 98, 117, 53, 57, 77, 118, 122, 99, 109, 84, 88, 98, 106, 115, 105, 55, 72, 89, 54, 122, 100, 53, 51, 89, 113, 53, 75, 50, 52, 52, 102, 119, 70, 72, 82, 81, 56, 101, 79, 66, 48, 73, 87, 66, 43, 52, 80, 102, 77, 55, 70, 101, 65, 65, 112, 90, 118, 108, 102, 113, 108, 75, 79, 108, 76, 99, 90, 76, 50, 117, 121, 86, 109, 122, 82, 107, 121, 82, 53, 121, 87, 55, 50, 117, 111, 57, 109, 101, 104, 88, 52, 52, 67, 105, 80, 74, 50, 102, 115, 101, 57, 89, 54, 101, 81, 116, 99, 102, 69, 104, 77, 80, 107, 109, 72, 88, 73, 48, 49, 115, 78, 43, 75, 119, 80, 98, 112, 65, 51, 57, 43, 120, 79, 115, 83, 116, 106, 104, 80, 57, 78, 49, 89, 49, 97, 50, 116, 81, 65, 86, 111, 43, 121, 86, 103, 76, 103, 86, 50, 72, 119, 115, 55, 51, 70, 99, 48, 111, 51, 119, 67, 55, 56, 113, 80, 69, 65, 43, 118, 50, 97, 82, 115, 47, 66, 101, 51, 90, 70, 68, 103, 68, 121, 103, 104, 99, 47, 49, 102, 103, 85, 43, 55, 67, 43, 80, 54, 107, 98, 113, 100, 52, 112, 111, 121, 98, 54, 73, 87, 56, 75, 67, 74, 98, 120, 102, 77, 74, 118, 107, 111, 114, 100, 78, 79, 103, 79, 85, 85, 120, 110, 100, 80, 72, 69, 105, 47, 116, 98, 47, 85, 55, 117, 76, 106, 76, 79, 103, 80, 65, 61, 61].to_vec().try_into().unwrap();
//     let ias_sig: ReportSign = [111, 117, 80, 86, 83, 122, 54, 66, 89, 121, 86, 83, 108, 73, 71, 104, 116, 120, 57, 66, 70, 120, 43, 54, 84, 106, 105, 121, 80, 48, 97, 119, 114, 101, 74, 83, 86, 51, 111, 65, 55, 69, 84, 67, 122, 82, 67, 101, 52, 53, 115, 77, 81, 66, 52, 78, 57, 48, 105, 115, 111, 88, 86, 52, 119, 68, 74, 86, 71, 71, 71, 76, 113, 56, 99, 70, 65, 116, 48, 52, 50, 53, 57, 90, 80, 75, 111, 70, 111, 122, 77, 83, 110, 77, 111, 119, 118, 87, 113, 54, 56, 78, 68, 112, 108, 85, 105, 69, 122, 90, 118, 106, 77, 99, 82, 111, 89, 81, 75, 54, 105, 108, 117, 77, 87, 66, 98, 85, 120, 119, 89, 52, 85, 80, 43, 83, 71, 82, 108, 84, 85, 110, 84, 49, 56, 87, 100, 79, 104, 71, 71, 122, 69, 79, 78, 115, 86, 43, 69, 107, 114, 47, 48, 80, 103, 57, 81, 99, 115, 49, 84, 120, 71, 86, 43, 49, 70, 57, 120, 51, 74, 118, 90, 87, 98, 75, 84, 66, 53, 82, 47, 50, 109, 113, 77, 115, 67, 100, 105, 112, 120, 109, 120, 73, 55, 75, 65, 53, 106, 82, 113, 57, 47, 67, 77, 68, 102, 111, 52, 72, 122, 86, 79, 106, 110, 78, 118, 122, 88, 51, 110, 83, 69, 100, 122, 108, 83, 65, 85, 117, 101, 105, 73, 87, 109, 51, 47, 89, 103, 51, 80, 69, 76, 72, 117, 118, 114, 115, 56, 116, 47, 73, 66, 122, 122, 104, 47, 72, 81, 114, 52, 104, 106, 81, 68, 119, 111, 87, 65, 108, 51, 86, 97, 76, 106, 105, 107, 68, 81, 49, 75, 48, 122, 54, 90, 82, 88, 82, 79, 70, 103, 114, 97, 73, 68, 43, 112, 54, 65, 57, 68, 116, 84, 74, 119, 97, 68, 83, 101, 102, 78, 82, 97, 65, 120, 51, 87, 69, 106, 122, 113, 43, 49, 103, 85, 78, 77, 116, 65, 85, 75, 103, 61, 61].to_vec().try_into().unwrap();

//     let ias_cert_dec = match base64::decode_config(ias_cert, base64::STANDARD) {
//         Ok(c) => c,
//         Err(_) => return,
//     };
//     let sig_cert: webpki::EndEntityCert = match webpki::EndEntityCert::from(ias_cert_dec.as_slice()) {
//         Ok(c) => c,
//         Err(_) => return,
//     };

//     let intermediate_report: Vec<&[u8]> = Vec::new();
//     //2022-12-09 00:00:00
//     let now_func = webpki::Time::from_seconds_since_unix_epoch(1670515200);

//     if let Err(_e) = sig_cert.verify_is_valid_tls_server_cert(
//         SUPPORTED_SIG_ALGS,
//         &IAS_SERVER_ROOTS,
//         &intermediate_report,
//         now_func
//     ) {return}

//     let ias_sig_dec: Vec<u8> = match base64::decode(ias_sig) {
//         Ok(value) => value,
//         Err(_) => return,
//     };

//     if let Err(_e) = sig_cert.verify_signature(
//         &webpki::RSA_PKCS1_2048_8192_SHA256,
//         &report_json_raw,
//         &ias_sig_dec,
//     ) {return}

//     println!("start test anaylize");

//     let some_quote_body: Value = match serde_json::from_slice(&report_json_raw) {
//         Ok(body) => body,
//         Err(_) => return,
//     };

//     if let Value::String(maybe_isv_quote_body) = &some_quote_body["isvEnclaveQuoteBody"] {
//         let decoded_quote_body = match base64::decode(&maybe_isv_quote_body) {
//             Ok(decoded_qb) => decoded_qb,
//             Err(_) => return,
//         };

//         let quote_report: QuoteReport = QuoteReport::try_from(decoded_quote_body.as_slice()).unwrap();

//         println!("quote_report mr_enclave:{:?}", quote_report.report_body.mr_enclave);
//         println!("quote_report mr_signer:{:?}", quote_report.report_body.mr_signer);
//         println!("quote_report report_data:{:?}", quote_report.report_body.report_data);
//         // let text = &quote_report.report_body.report_data[0..64];
//         // println!("quote_report report_data slice:{:?}", text);
//         // println!("quote_report report_data slice convert to bytes:{:?}, len:{:?}", text.as_bytes(), text.as_bytes().len());
//         // let text = sp_io::hashing::sha2_256(text.as_bytes());
//         // let text_hex_str = hex::encode(text);
//         // println!("quote_report report_data slice hashing:{:?}, len:{:?}", text, text.len());
//         // println!("quote_report report_data text_hex_str hashing:{:?}, text_hex_str length:{:?}", text_hex_str, text_hex_str.len());
//     }
//     // Some(1)
// }

// use sp_core::ConstU32;
// #[derive(PartialEq, Eq, Encode, Decode, Clone)]
// pub struct TestVerifyIdleResultInfo {
// 	pub miner_prove: BoundedVec<u8, ConstU32<1024>>,
// 	pub front: u64,
// 	pub rear: u64,
// 	pub accumulator: [u8; 256],
// 	pub space_challenge_param: [u64; 8],
// 	// pub result: bool,
// }

// #[test]
// fn test_print_encode() {
// 	let verify_idle_result = TestVerifyIdleResultInfo {
// 		miner_prove: hex::decode("99dfa1360614be65f30d0fe13918f65850eecf468ce7518522854820e352cb25".as_bytes().to_vec()).unwrap().try_into().unwrap(),
// 		front: 0,
// 		rear: 256,
// 		accumulator: hex::decode("62f9f119a95fab3845e57a363de6becabd1303ddb060b0dfab8cdf0013adabf2e9182bcd198d70a1f369ad73971ac34af44ef22b0a76a70e90c8c2626d065757e54a0a11a011d064a756b8c5b391426ca1bba13877bc3b8899b8b54c4766715edf42cb53897620906d81b8a7012798b6b35c76021f37511df0f4f5fb574438c674facfd74de51c811028960dde1e7532559d2ddf627912ef25822d05d73593cd859eda425e87e0a91d2e06b273f241f7f8b778c72529c7c233f676e017f1885628f137e24b1795eff8f870bb3c31e1aa0ac93fea9d49b0ca80698ce3c2dfb37f681f9d4bd8a9b7b9c1ca5070dbf333338a5e29b825e34bf73955694f483e9088".as_bytes().to_vec()).unwrap().try_into().unwrap(),
// 		space_challenge_param: [1049639, 1052941, 1052891, 1062842, 1053236, 1056496, 1062998, 1064471],
// 	 	// result: true,
// 	};

// 	// log::info!("acc: {:?}", verify_idle_result.miner);
// 	println!("miner_prove: {:?}", verify_idle_result.miner_prove);
// 	println!("front: {:?}", verify_idle_result.front);
// 	println!("rear: {:?}", verify_idle_result.rear);
// 	println!("accumulator: {:?}", verify_idle_result.accumulator);
// 	println!("space_challenge_param: {:?}", verify_idle_result.space_challenge_param);
// 	// println!("result: {:?}", verify_idle_result.result);

// 	// let tee_puk = T::TeeWorkerHandler::get_tee_publickey()?;
// 	let encoding = verify_idle_result.encode();
// 	println!("encoding: {:?}", encoding);
// 	let hashing = sp_io::hashing::sha2_256(&encoding);
// 	println!("hashing: {:?}", hashing);

//     let nnn = "5864462488183348501199387698765396512289232955242182008827944582139908780909797731429665563976952159440759673602740200628448155762035850744908361942163219788707342849276956219272557300944946213898958304606806594531530758964311352865973133862781754596266278147403773996275034177101872377977010783717374901962788773872176972605124456107312770317160788203244286191844710735579967504716696421669633134548036263828604158473325056195771945773017116322291719389653804514256715194895532499003173681383320608508670796452789413344655304726798876843616282008382839350334839526656218332939237947291803078918002361299720745985196".as_bytes();
    
//     println!("nnn length: {:?}, nnn: {:?}", nnn.len(), nnn);

//     // let arr1 = [128, 175, 235, 79, 69, 236, 151, 1, 63, 10, 80, 101, 15, 104, 102, 68, 82, 102, 144, 31, 130, 37, 151, 141, 182, 33, 239, 44, 219, 211, 208, 17, 42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 98, 249, 241, 25, 169, 95, 171, 56, 69, 229, 122, 54, 61, 230, 190, 202, 189, 19, 3, 221, 176, 96, 176, 223, 171, 140, 223, 0, 19, 173, 171, 242, 233, 24, 43, 205, 25, 141, 112, 161, 243, 105, 173, 115, 151, 26, 195, 74, 244, 78, 242, 43, 10, 118, 167, 14, 144, 200, 194, 98, 109, 6, 87, 87, 229, 74, 10, 17, 160, 17, 208, 100, 167, 86, 184, 197, 179, 145, 66, 108, 161, 187, 161, 56, 119, 188, 59, 136, 153, 184, 181, 76, 71, 102, 113, 94, 223, 66, 203, 83, 137, 118, 32, 144, 109, 129, 184, 167, 1, 39, 152, 182, 179, 92, 118, 2, 31, 55, 81, 29, 240, 244, 245, 251, 87, 68, 56, 198, 116, 250, 207, 215, 77, 229, 28, 129, 16, 40, 150, 13, 222, 30, 117, 50, 85, 157, 45, 223, 98, 121, 18, 239, 37, 130, 45, 5, 215, 53, 147, 205, 133, 158, 218, 66, 94, 135, 224, 169, 29, 46, 6, 178, 115, 242, 65, 247, 248, 183, 120, 199, 37, 41, 199, 194, 51, 246, 118, 224, 23, 241, 136, 86, 40, 241, 55, 226, 75, 23, 149, 239, 248, 248, 112, 187, 60, 49, 225, 170, 10, 201, 63, 234, 157, 73, 176, 202, 128, 105, 140, 227, 194, 223, 179, 127, 104, 31, 157, 75, 216, 169, 183, 185, 193, 202, 80, 112, 219, 243, 51, 51, 138, 94, 41, 184, 37, 227, 75, 247, 57, 85, 105, 79, 72, 62, 144, 136];
//     // let arr2 = [128, 95, 217, 26, 235, 41, 133, 237, 24, 226, 137, 158, 59, 212, 254, 56, 90, 141, 190, 23, 81, 44, 56, 228, 4, 24, 138, 220, 106, 185, 238, 219, 239, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 98, 249, 241, 25, 169, 95, 171, 56, 69, 229, 122, 54, 61, 230, 190, 202, 189, 19, 3, 221, 176, 96, 176, 223, 171, 140, 223, 0, 19, 173, 171, 242, 233, 24, 43, 205, 25, 141, 112, 161, 243, 105, 173, 115, 151, 26, 195, 74, 244, 78, 242, 43, 10, 118, 167, 14, 144, 200, 194, 98, 109, 6, 87, 87, 229, 74, 10, 17, 160, 17, 208, 100, 167, 86, 184, 197, 179, 145, 66, 108, 161, 187, 161, 56, 119, 188, 59, 136, 153, 184, 181, 76, 71, 102, 113, 94, 223, 66, 203, 83, 137, 118, 32, 144, 109, 129, 184, 167, 1, 39, 152, 182, 179, 92, 118, 2, 31, 55, 81, 29, 240, 244, 245, 251, 87, 68, 56, 198, 116, 250, 207, 215, 77, 229, 28, 129, 16, 40, 150, 13, 222, 30, 117, 50, 85, 157, 45, 223, 98, 121, 18, 239, 37, 130, 45, 5, 215, 53, 147, 205, 133, 158, 218, 66, 94, 135, 224, 169, 29, 46, 6, 178, 115, 242, 65, 247, 248, 183, 120, 199, 37, 41, 199, 194, 51, 246, 118, 224, 23, 241, 136, 86, 40, 241, 55, 226, 75, 23, 149, 239, 248, 248, 112, 187, 60, 49, 225, 170, 10, 201, 63, 234, 157, 73, 176, 202, 128, 105, 140, 227, 194, 223, 179, 127, 104, 31, 157, 75, 216, 169, 183, 185, 193, 202, 80, 112, 219, 243, 51, 51, 138, 94, 41, 184, 37, 227, 75, 247, 57, 85, 105, 79, 72, 62, 144, 136, 32];
//     // assert_eq!(arr1, arr2);
// 	// ensure!(verify_rsa(&tee_puk, &hashing, &signature), Error::<T>::VerifyTeeSigFailed);
// }
// #[derive(PartialEq, Eq, Encode, Decode, Clone)]
// pub struct VerifyServiceResultInfo {
// 	pub miner_prove: BoundedVec<u8, ConstU32<u32>>,
// 	pub result: bool,
// 	pub chal: QElement,
// 	pub service_bloom_filter: [u64; 256],
// }

// pub struct QElement {
// 	pub random_index_list: BoundedVec<u32, ConstU32<1024>>,
// 	pub random_list: BoundedVec<[u8; 20], ConstU32<1024>>,
// }

// #[test]
// fn test_print_encode() {
//     let verify_service_result_info = VerifyServiceResultInfo {
//         miner_prove: ,
//         result: true,
//         chal: ,
//         service_bloom_filter: ,
//     }

//     println!("miner_prove: {:?}", verify_service_result_info.miner_prove.encode());
//     println!("chal: {:?}", verify_service_result_info.chal.encode());
//     println!("service_bloom_filter: {:?}", verify_service_result_info.service_bloom_filter.encode());

//     let encoding = verify_service_result_info.encode();
// 	println!("encoding: {:?}", encoding);
// 	let hashing = sp_io::hashing::sha2_256(&encoding);
// 	println!("hashing: {:?}", hashing);
// }
