//! Ephemeral CA and certificate generation for mTLS.
//!
//! The supervisor generates a CA at startup. Each workspace gets a
//! client cert signed by the CA. The cert encodes the agent_id and
//! role. Certs and CA are ephemeral — they die with the supervisor.

use std::sync::Arc;

use rcgen::{
    BasicConstraints, CertificateParams, DnType, IsCa, KeyPair, KeyUsagePurpose,
};

/// An ephemeral certificate authority.
pub struct EphemeralCA {
    pub ca_cert_pem: String,
    pub ca_key_pem: String,
    ca_cert: rcgen::Certificate,
    ca_key: KeyPair,
}

/// A signed client certificate for a workspace.
pub struct WorkspaceCert {
    pub cert_pem: String,
    pub key_pem: String,
}

impl EphemeralCA {
    /// Reconstruct a CA from PEM-encoded cert and key.
    /// Used by coordinators to load the supervisor's CA.
    ///
    /// Re-creates the CA cert parameters and self-signs with the same key.
    /// The resulting CA can sign workspace certs that validate against the
    /// original supervisor CA (same key = same signature verification).
    pub fn from_pem(
        _ca_cert_pem: &str,
        ca_key_pem: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let key = KeyPair::from_pem(ca_key_pem)?;

        // Recreate the same CA parameters the supervisor used.
        let mut params = CertificateParams::new(Vec::<String>::new())?;
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params
            .distinguished_name
            .push(DnType::CommonName, "clc-supervisor-ca");
        params
            .distinguished_name
            .push(DnType::OrganizationName, "clc");
        params.key_usages.push(KeyUsagePurpose::KeyCertSign);
        params.key_usages.push(KeyUsagePurpose::CrlSign);

        let ca_cert = params.self_signed(&key)?;
        let ca_cert_pem = ca_cert.pem();
        let ca_key_pem_owned = key.serialize_pem();

        Ok(Self {
            ca_cert_pem,
            ca_key_pem: ca_key_pem_owned,
            ca_cert,
            ca_key: key,
        })
    }

    /// Generate a new ephemeral CA. Called once at supervisor startup.
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut params = CertificateParams::new(Vec::<String>::new())?;
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params
            .distinguished_name
            .push(DnType::CommonName, "clc-supervisor-ca");
        params
            .distinguished_name
            .push(DnType::OrganizationName, "clc");
        params.key_usages.push(KeyUsagePurpose::KeyCertSign);
        params.key_usages.push(KeyUsagePurpose::CrlSign);

        let key = KeyPair::generate()?;
        let ca_cert = params.self_signed(&key)?;
        let ca_cert_pem = ca_cert.pem();
        let ca_key_pem = key.serialize_pem();

        Ok(Self {
            ca_cert_pem,
            ca_key_pem,
            ca_cert,
            ca_key: key,
        })
    }

    /// Sign a client certificate for a workspace agent.
    pub fn sign_workspace_cert(
        &self,
        agent_id: &str,
        role: &str,
    ) -> Result<WorkspaceCert, Box<dyn std::error::Error>> {
        let mut params = CertificateParams::new(Vec::<String>::new())?;
        params
            .distinguished_name
            .push(DnType::CommonName, agent_id);
        params
            .distinguished_name
            .push(DnType::OrganizationalUnitName, role);

        let key = KeyPair::generate()?;
        let cert = params.signed_by(&key, &self.ca_cert, &self.ca_key)?;

        Ok(WorkspaceCert {
            cert_pem: cert.pem(),
            key_pem: key.serialize_pem(),
        })
    }

    /// Build a rustls ServerConfig that requires client certs signed by this CA.
    #[allow(dead_code)]
    pub fn server_tls_config(&self) -> Result<Arc<rustls::ServerConfig>, Box<dyn std::error::Error>> {
        use rustls::pki_types::CertificateDer;
        use rustls::server::WebPkiClientVerifier;

        // Parse CA cert for client verification.
        let ca_cert_der = CertificateDer::from(self.ca_cert.der().to_vec());
        let mut root_store = rustls::RootCertStore::empty();
        root_store.add(ca_cert_der)?;

        let client_verifier = WebPkiClientVerifier::builder(Arc::new(root_store)).build()?;

        // Server cert (self-signed, used for the TLS handshake).
        let mut server_params = CertificateParams::new(vec![
            "localhost".to_string(),
        ])?;
        server_params
            .distinguished_name
            .push(DnType::CommonName, "clc-supervisor-api");
        let server_key = KeyPair::generate()?;
        let server_cert = server_params.signed_by(&server_key, &self.ca_cert, &self.ca_key)?;

        let server_cert_der = CertificateDer::from(server_cert.der().to_vec());
        let server_key_der = rustls::pki_types::PrivateKeyDer::Pkcs8(
            rustls::pki_types::PrivatePkcs8KeyDer::from(server_key.serialize_der()),
        );

        // Include the CA cert in the server's cert chain so clients can
        // verify the server cert without having the CA pre-installed.
        let ca_cert_for_chain = CertificateDer::from(self.ca_cert.der().to_vec());
        let config = rustls::ServerConfig::builder()
            .with_client_cert_verifier(client_verifier)
            .with_single_cert(vec![server_cert_der, ca_cert_for_chain], server_key_der)?;

        Ok(Arc::new(config))
    }
}
