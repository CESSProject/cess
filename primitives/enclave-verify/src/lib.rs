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
use sp_core::bounded::BoundedVec;
use ic_verify_bls_signature::{
    Signature as BLSSignature,
    PublicKey as BLSPubilc,
};

#[cfg(test)]
use rand::RngCore;

#[cfg(test)]
use rsa::{
    RsaPublicKey, RsaPrivateKey, 
    pkcs1::{EncodeRsaPrivateKey, DecodeRsaPrivateKey},
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
// #[test]
// fn cryptos_rsa() {
// 	// let mut rng = rand::thread_rng();
//     let priv_key_der = [48, 130, 4, 164, 2, 1, 0, 2, 130, 1, 1, 0, 232, 164, 71, 61, 156, 19, 143, 219, 24, 119, 196, 115, 203, 33, 130, 151, 75, 193, 108, 45, 181, 28, 191, 217, 102, 16, 251, 69, 156, 138, 34, 69, 68, 239, 167, 157, 121, 34, 146, 221, 122, 71, 183, 180, 190, 53, 5, 94, 154, 224, 178, 215, 143, 210, 96, 209, 51, 13, 153, 186, 209, 42, 30, 184, 117, 100, 112, 165, 234, 132, 48, 11, 137, 160, 143, 171, 209, 37, 93, 58, 237, 199, 4, 65, 231, 156, 171, 238, 184, 196, 182, 185, 5, 15, 216, 174, 194, 238, 247, 101, 25, 167, 108, 61, 236, 131, 208, 221, 104, 49, 198, 233, 98, 40, 30, 35, 0, 99, 93, 169, 190, 225, 76, 106, 55, 179, 135, 252, 71, 124, 215, 70, 189, 104, 167, 157, 31, 169, 7, 65, 147, 103, 47, 238, 62, 44, 136, 49, 31, 68, 176, 103, 77, 230, 83, 205, 162, 237, 154, 196, 193, 246, 79, 40, 206, 156, 87, 70, 178, 11, 64, 59, 174, 248, 210, 233, 140, 144, 93, 197, 115, 32, 133, 44, 157, 125, 226, 159, 221, 4, 19, 26, 247, 234, 54, 49, 216, 114, 142, 130, 13, 163, 250, 178, 72, 32, 187, 175, 59, 189, 53, 174, 19, 252, 169, 83, 235, 175, 38, 76, 241, 124, 131, 86, 46, 38, 87, 119, 45, 101, 51, 6, 133, 36, 178, 123, 212, 137, 57, 14, 110, 20, 164, 219, 79, 134, 82, 98, 213, 246, 115, 68, 119, 104, 157, 209, 2, 3, 1, 0, 1, 2, 130, 1, 0, 20, 199, 199, 205, 75, 9, 188, 73, 215, 207, 170, 238, 164, 240, 99, 87, 220, 94, 116, 169, 72, 138, 62, 224, 206, 107, 41, 230, 183, 234, 230, 208, 197, 45, 155, 13, 71, 234, 188, 175, 167, 226, 140, 24, 74, 253, 53, 115, 147, 230, 10, 83, 146, 247, 57, 202, 182, 2, 186, 254, 162, 252, 94, 46, 31, 222, 78, 233, 163, 31, 23, 163, 144, 49, 149, 99, 197, 148, 206, 213, 26, 180, 50, 63, 40, 207, 39, 212, 117, 16, 173, 57, 173, 168, 18, 180, 217, 152, 186, 228, 126, 252, 35, 129, 12, 133, 97, 188, 197, 55, 221, 10, 175, 199, 225, 153, 66, 201, 157, 110, 50, 44, 177, 196, 179, 33, 8, 62, 103, 16, 113, 237, 189, 74, 15, 207, 163, 190, 126, 165, 68, 160, 205, 187, 209, 164, 50, 182, 36, 117, 189, 168, 74, 209, 34, 132, 203, 189, 24, 142, 239, 235, 57, 108, 35, 67, 185, 29, 111, 17, 89, 174, 251, 96, 23, 178, 250, 17, 235, 126, 7, 56, 209, 94, 234, 116, 38, 125, 170, 184, 100, 109, 159, 239, 202, 150, 66, 134, 112, 25, 158, 242, 240, 127, 21, 111, 191, 144, 105, 84, 108, 220, 92, 86, 99, 255, 210, 145, 238, 175, 219, 57, 178, 44, 216, 130, 34, 214, 180, 141, 106, 201, 114, 245, 89, 255, 99, 203, 35, 50, 36, 12, 219, 9, 115, 221, 30, 223, 36, 161, 39, 155, 84, 121, 1, 25, 2, 129, 129, 0, 238, 166, 41, 103, 247, 201, 251, 212, 146, 222, 200, 89, 157, 94, 61, 144, 175, 207, 151, 114, 104, 132, 125, 136, 189, 151, 128, 32, 180, 50, 4, 79, 214, 240, 61, 163, 43, 61, 246, 4, 22, 24, 120, 187, 216, 12, 242, 92, 166, 52, 109, 251, 169, 104, 51, 193, 103, 117, 39, 172, 251, 175, 226, 82, 183, 73, 64, 206, 131, 241, 182, 154, 179, 129, 249, 104, 97, 128, 208, 140, 181, 17, 192, 208, 183, 31, 94, 83, 245, 10, 208, 185, 54, 102, 198, 35, 130, 36, 239, 233, 236, 174, 140, 107, 4, 205, 15, 73, 94, 238, 141, 210, 203, 106, 105, 20, 21, 230, 149, 218, 60, 182, 34, 55, 238, 8, 80, 163, 2, 129, 129, 0, 249, 142, 78, 23, 123, 50, 222, 201, 91, 71, 233, 52, 66, 98, 193, 2, 137, 51, 187, 7, 87, 220, 46, 104, 75, 128, 47, 41, 209, 0, 37, 4, 130, 83, 61, 4, 176, 148, 218, 20, 219, 217, 53, 41, 97, 150, 164, 129, 71, 13, 106, 82, 251, 226, 204, 102, 194, 25, 212, 216, 105, 240, 245, 123, 136, 95, 109, 75, 170, 181, 186, 243, 141, 2, 206, 53, 201, 163, 117, 237, 236, 45, 16, 243, 0, 53, 179, 174, 153, 107, 42, 186, 234, 219, 199, 59, 121, 194, 117, 242, 53, 198, 130, 159, 114, 192, 51, 32, 93, 45, 233, 47, 242, 210, 219, 8, 65, 209, 254, 164, 155, 175, 232, 63, 178, 187, 26, 251, 2, 129, 128, 62, 7, 62, 55, 225, 181, 196, 24, 202, 91, 209, 99, 73, 125, 215, 46, 166, 35, 164, 207, 125, 207, 1, 249, 234, 157, 88, 22, 39, 255, 224, 19, 8, 96, 197, 4, 134, 22, 194, 188, 233, 41, 79, 40, 51, 205, 153, 168, 239, 34, 45, 123, 253, 218, 49, 169, 145, 68, 104, 29, 148, 5, 113, 35, 226, 179, 205, 126, 95, 217, 17, 135, 64, 37, 6, 56, 85, 47, 112, 5, 66, 130, 236, 196, 210, 243, 250, 70, 132, 40, 93, 123, 230, 97, 236, 26, 10, 151, 163, 43, 255, 242, 150, 88, 178, 148, 193, 230, 102, 32, 71, 8, 133, 10, 145, 105, 65, 15, 255, 223, 11, 108, 163, 148, 57, 240, 59, 85, 2, 129, 129, 0, 169, 39, 34, 27, 156, 112, 64, 190, 111, 86, 240, 229, 113, 82, 10, 205, 179, 62, 19, 57, 200, 253, 255, 158, 197, 254, 94, 249, 147, 38, 235, 240, 128, 125, 247, 80, 36, 120, 224, 209, 94, 171, 125, 243, 76, 168, 149, 92, 227, 82, 94, 141, 93, 26, 191, 189, 175, 55, 95, 36, 73, 187, 0, 73, 249, 135, 229, 71, 114, 176, 183, 197, 186, 0, 250, 209, 78, 153, 179, 167, 207, 124, 68, 142, 209, 199, 148, 193, 118, 80, 67, 168, 106, 229, 9, 200, 112, 161, 180, 220, 182, 66, 149, 235, 138, 22, 105, 17, 56, 215, 147, 197, 226, 107, 181, 247, 132, 213, 216, 42, 175, 52, 174, 209, 238, 78, 16, 221, 2, 129, 129, 0, 148, 213, 2, 173, 34, 234, 43, 241, 127, 238, 197, 184, 144, 188, 231, 126, 80, 124, 90, 35, 230, 23, 188, 232, 27, 30, 176, 249, 43, 120, 117, 237, 196, 228, 228, 209, 49, 231, 254, 97, 129, 128, 82, 117, 119, 16, 66, 168, 21, 163, 127, 132, 67, 188, 133, 81, 140, 89, 26, 44, 194, 196, 123, 52, 72, 219, 91, 48, 10, 227, 230, 149, 125, 59, 111, 80, 121, 253, 252, 63, 193, 131, 193, 254, 131, 245, 169, 6, 152, 86, 51, 56, 153, 22, 235, 89, 161, 156, 128, 86, 133, 4, 88, 233, 131, 134, 38, 144, 127, 221, 159, 112, 209, 2, 212, 85, 165, 56, 89, 26, 122, 49, 150, 246, 172, 129, 8, 5];
//     // let bits = 2048;
//     // let priv_key = RsaPrivateKey::new(&mut rng, bits).expect("failed to generate a key");
//     let priv_key = rsa::RsaPrivateKey::from_pkcs1_der(&priv_key_der).expect("failed to generate a key");

// 	let pub_key = RsaPublicKey::from(&priv_key);

//     let pois_key = PoISKey {
//         g: [2u8; 256],
//         n: [3u8; 256],
//     };

//     let encoding = pois_key.encode();
//     let original_text = [39, 237, 28, 25, 198, 199, 111, 216, 246, 132, 99, 17, 147, 167, 12, 125, 19, 3, 49, 230, 142, 18, 75, 255, 210, 134, 56, 103, 140, 148, 21, 15];

// 	let priv_key_der = priv_key.to_pkcs1_der().unwrap();
//     // println!("priv_key_der: {:?}", priv_key_der.as_bytes());
//     // println!();

// 	let doc = pub_key.to_pkcs1_der().unwrap();
//     // println!("pub_key: {:?}, length: {}", doc.as_bytes(), doc.as_bytes().len());

//     let pk = rsa::RsaPublicKey::from_pkcs1_der(doc.as_bytes()).unwrap();

// 	let msg = "hello world!".as_bytes();
// 	let binding = priv_key.sign(Pkcs1v15Sign::new_raw(), &original_text).unwrap();
//     let sig: &[u8] = binding.as_ref();
//     println!("sig is: {:?}, sig is length: {:?}", sig, sig.len());

//     let result = pk.verify(Pkcs1v15Sign::new_raw(), &msg, &sig);

// 	println!("result: {:?}", result);
// }
use sp_core::ConstU32;
#[derive(PartialEq, Eq, Encode, Decode, Clone)]
pub struct TestVerifyIdleResultInfo {
	pub miner_prove: BoundedVec<u8, ConstU32<1024>>,
	pub front: u64,
	pub rear: u64,
	pub accumulator: [u8; 256],
	pub space_challenge_param: [u64; 8],
	// pub result: bool,
}

#[test]
fn test_print_encode() {
	let verify_idle_result = TestVerifyIdleResultInfo {
		miner_prove: hex::decode("99dfa1360614be65f30d0fe13918f65850eecf468ce7518522854820e352cb25".as_bytes().to_vec()).unwrap().try_into().unwrap(),
		front: 0,
		rear: 256,
		accumulator: hex::decode("62f9f119a95fab3845e57a363de6becabd1303ddb060b0dfab8cdf0013adabf2e9182bcd198d70a1f369ad73971ac34af44ef22b0a76a70e90c8c2626d065757e54a0a11a011d064a756b8c5b391426ca1bba13877bc3b8899b8b54c4766715edf42cb53897620906d81b8a7012798b6b35c76021f37511df0f4f5fb574438c674facfd74de51c811028960dde1e7532559d2ddf627912ef25822d05d73593cd859eda425e87e0a91d2e06b273f241f7f8b778c72529c7c233f676e017f1885628f137e24b1795eff8f870bb3c31e1aa0ac93fea9d49b0ca80698ce3c2dfb37f681f9d4bd8a9b7b9c1ca5070dbf333338a5e29b825e34bf73955694f483e9088".as_bytes().to_vec()).unwrap().try_into().unwrap(),
		space_challenge_param: [1049639, 1052941, 1052891, 1062842, 1053236, 1056496, 1062998, 1064471],
	 	// result: true,
	};

	// log::info!("acc: {:?}", verify_idle_result.miner);
	println!("miner_prove: {:?}", verify_idle_result.miner_prove);
	println!("front: {:?}", verify_idle_result.front);
	println!("rear: {:?}", verify_idle_result.rear);
	println!("accumulator: {:?}", verify_idle_result.accumulator);
	println!("space_challenge_param: {:?}", verify_idle_result.space_challenge_param);
	// println!("result: {:?}", verify_idle_result.result);

	// let tee_puk = T::TeeWorkerHandler::get_tee_publickey()?;
	let encoding = verify_idle_result.encode();
	println!("encoding: {:?}", encoding);
	let hashing = sp_io::hashing::sha2_256(&encoding);
	println!("hashing: {:?}", hashing);

    // let arr1 = [128, 175, 235, 79, 69, 236, 151, 1, 63, 10, 80, 101, 15, 104, 102, 68, 82, 102, 144, 31, 130, 37, 151, 141, 182, 33, 239, 44, 219, 211, 208, 17, 42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 98, 249, 241, 25, 169, 95, 171, 56, 69, 229, 122, 54, 61, 230, 190, 202, 189, 19, 3, 221, 176, 96, 176, 223, 171, 140, 223, 0, 19, 173, 171, 242, 233, 24, 43, 205, 25, 141, 112, 161, 243, 105, 173, 115, 151, 26, 195, 74, 244, 78, 242, 43, 10, 118, 167, 14, 144, 200, 194, 98, 109, 6, 87, 87, 229, 74, 10, 17, 160, 17, 208, 100, 167, 86, 184, 197, 179, 145, 66, 108, 161, 187, 161, 56, 119, 188, 59, 136, 153, 184, 181, 76, 71, 102, 113, 94, 223, 66, 203, 83, 137, 118, 32, 144, 109, 129, 184, 167, 1, 39, 152, 182, 179, 92, 118, 2, 31, 55, 81, 29, 240, 244, 245, 251, 87, 68, 56, 198, 116, 250, 207, 215, 77, 229, 28, 129, 16, 40, 150, 13, 222, 30, 117, 50, 85, 157, 45, 223, 98, 121, 18, 239, 37, 130, 45, 5, 215, 53, 147, 205, 133, 158, 218, 66, 94, 135, 224, 169, 29, 46, 6, 178, 115, 242, 65, 247, 248, 183, 120, 199, 37, 41, 199, 194, 51, 246, 118, 224, 23, 241, 136, 86, 40, 241, 55, 226, 75, 23, 149, 239, 248, 248, 112, 187, 60, 49, 225, 170, 10, 201, 63, 234, 157, 73, 176, 202, 128, 105, 140, 227, 194, 223, 179, 127, 104, 31, 157, 75, 216, 169, 183, 185, 193, 202, 80, 112, 219, 243, 51, 51, 138, 94, 41, 184, 37, 227, 75, 247, 57, 85, 105, 79, 72, 62, 144, 136];
    // let arr2 = [128, 95, 217, 26, 235, 41, 133, 237, 24, 226, 137, 158, 59, 212, 254, 56, 90, 141, 190, 23, 81, 44, 56, 228, 4, 24, 138, 220, 106, 185, 238, 219, 239, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 98, 249, 241, 25, 169, 95, 171, 56, 69, 229, 122, 54, 61, 230, 190, 202, 189, 19, 3, 221, 176, 96, 176, 223, 171, 140, 223, 0, 19, 173, 171, 242, 233, 24, 43, 205, 25, 141, 112, 161, 243, 105, 173, 115, 151, 26, 195, 74, 244, 78, 242, 43, 10, 118, 167, 14, 144, 200, 194, 98, 109, 6, 87, 87, 229, 74, 10, 17, 160, 17, 208, 100, 167, 86, 184, 197, 179, 145, 66, 108, 161, 187, 161, 56, 119, 188, 59, 136, 153, 184, 181, 76, 71, 102, 113, 94, 223, 66, 203, 83, 137, 118, 32, 144, 109, 129, 184, 167, 1, 39, 152, 182, 179, 92, 118, 2, 31, 55, 81, 29, 240, 244, 245, 251, 87, 68, 56, 198, 116, 250, 207, 215, 77, 229, 28, 129, 16, 40, 150, 13, 222, 30, 117, 50, 85, 157, 45, 223, 98, 121, 18, 239, 37, 130, 45, 5, 215, 53, 147, 205, 133, 158, 218, 66, 94, 135, 224, 169, 29, 46, 6, 178, 115, 242, 65, 247, 248, 183, 120, 199, 37, 41, 199, 194, 51, 246, 118, 224, 23, 241, 136, 86, 40, 241, 55, 226, 75, 23, 149, 239, 248, 248, 112, 187, 60, 49, 225, 170, 10, 201, 63, 234, 157, 73, 176, 202, 128, 105, 140, 227, 194, 223, 179, 127, 104, 31, 157, 75, 216, 169, 183, 185, 193, 202, 80, 112, 219, 243, 51, 51, 138, 94, 41, 184, 37, 227, 75, 247, 57, 85, 105, 79, 72, 62, 144, 136, 32];
    // assert_eq!(arr1, arr2);
	// ensure!(verify_rsa(&tee_puk, &hashing, &signature), Error::<T>::VerifyTeeSigFailed);
}