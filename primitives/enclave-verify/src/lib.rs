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
    pkcs8::DecodePublicKey,
};

#[test]
use rand::RngCore;

#[test]
use rsa::{
    RsaPublicKey, RsaPrivateKey, 
    EncodePublicKey,
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
    let pk = rsa::RsaPublicKey::from_public_key_der(key).unwrap();

    match pk.verify(Pkcs1v15Sign::new_raw(), msg, sig) {
        Ok(()) => return true,
        Err(_) => return false,
    };
}

// pub fn sig_rsa(key: &[u8], msg: &[u8]) -> &[u8] {

// }


#[test]
fn cryptos_rsa() {
	let mut rng = rand::thread_rng();
	let bits = 2048;
	let priv_key = RsaPrivateKey::new(&mut rng, bits).expect("failed to generate a key");
	let pub_key = RsaPublicKey::from(&priv_key);

	let doc = pub_key.to_public_key_der().unwrap();
	let msg = "hello world!".as_bytes();
	let binding = priv_key.sign(Pkcs1v15Sign::new_raw(), msg).unwrap();
    let sig = binding.as_ref();
	let result = verify_rsa(&doc.as_bytes(), &msg, &sig);
	println!("result: {:?}", result);
}

#[test]
fn first_char_roll() {
    let mut rng = rand::thread_rng();
    println!("si ling fa shi: {:?}", rng.next_u32() % 100);
    println!("de lu yi: {:?}", rng.next_u32() % 100);
    println!("ye man ren: {:?}", rng.next_u32() % 100);
    println!("fa shi: {:?}", rng.next_u32() % 100);
    println!("you xia: {:?}", rng.next_u32() % 100);
}