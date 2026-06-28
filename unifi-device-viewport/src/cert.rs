use anyhow::Result;
use rcgen::{CertificateParams, DnType, ExtendedKeyUsagePurpose, KeyPair};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use time::Duration as TimeDuration;
use time::OffsetDateTime;

/// Generates a self-signed TLS server certificate and private key.
///
/// Mimics the factory-reset certificate generation on real Ubiquiti devices:
/// a self-signed cert with a long validity period (10 years) and server auth EKU.
/// Returns the DER-encoded certificate and PKCS#8 DER-encoded private key.
pub fn generate_self_signed_cert() -> Result<(CertificateDer<'static>, PrivateKeyDer<'static>)> {
    let mut params = CertificateParams::new(vec!["unifi-device-viewport".to_owned()])?;
    params
        .distinguished_name
        .push(DnType::CommonName, "unifi-device-viewport");
    params
        .extended_key_usages
        .push(ExtendedKeyUsagePurpose::ServerAuth);

    let now = OffsetDateTime::now_utc();
    params.not_before = now;
    params.not_after = now + TimeDuration::days(365 * 10);

    let key_pair = KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;

    let cert_der = CertificateDer::from(cert.der().to_vec());
    let key_der = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(key_pair.serialize_der()));

    Ok((cert_der, key_der))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_generate_cert_then_der_is_valid() {
        let (cert, key) = generate_self_signed_cert().unwrap();
        assert!(!cert.is_empty());
        assert!(!key.secret_der().is_empty());
    }
}
