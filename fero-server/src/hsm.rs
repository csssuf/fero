use failure::Error;
use libyubihsm::*;
use num::BigUint;
use pretty_good::{HashAlgorithm, PublicKeyAlgorithm, Signature, SignaturePacket, SignatureType};
use yasna;

#[derive(Clone, Debug)]
pub struct Hsm {
    yubihsm: Yubihsm,
    connector: Connector,
    session: Session,
    authkey: u16,
}

impl Hsm {
    pub fn new(connector_url: &str, authkey: u16, password: &str) -> Result<Hsm, Error> {
        let yubihsm = Yubihsm::new()?;
        let connector = yubihsm.connector().connect(connector_url)?;
        let session = connector.create_session_from_password(authkey, password, true)?;

        Ok(Hsm { yubihsm, connector, session, authkey })
    }

    fn create_digestinfo(payload: &[u8], hash_algo: HashAlgorithm) -> Result<Vec<u8>, Error> {
        let oid = hash_algo.asn1_oid()?;

        Ok(yasna::construct_der(|writer| {
            writer.write_sequence(|seq_writer| {
                seq_writer.next().write_sequence(|oid_seq_writer| {
                    oid_seq_writer.next().write_oid(&oid);
                    oid_seq_writer.next().write_null();
                });
                seq_writer.next().write_bytes(payload);
            });
        }))
    }

    pub fn create_pgp_signature<T: AsRef<[u8]>>(
        &self,
        payload: T,
        signing_key: u16,
        hash_algorithm: HashAlgorithm,
    ) -> Result<SignaturePacket, Error> {
        let mut sig_packet = SignaturePacket::new(
            SignatureType::BinaryDocument,
            PublicKeyAlgorithm::Rsa,
            hash_algorithm,
        )?;

        let signable_payload = sig_packet.signable_payload(payload)?;
        let digestinfo = Hsm::create_digestinfo(&signable_payload, hash_algorithm)?;

        let signature = self.session.sign_pkcs1v1_5(signing_key, false, digestinfo)?;
        sig_packet.set_contents(Signature::Rsa(BigUint::from_bytes_be(&signature)))?;

        Ok(sig_packet)
    }

    pub fn create_rsa_signature<T: AsRef<[u8]>>(
        &self,
        payload: T,
        signing_key: u16,
    ) -> Result<Vec<u8>, Error> {
        let payload = payload.as_ref();
        let hash_algorithm = match payload.len() {
            32 => HashAlgorithm::Sha256,
            48 => HashAlgorithm::Sha384,
            64 => HashAlgorithm::Sha512,
            _ => bail!("Payload must be a SHA256, SHA384 or SHA512 hash"),
        };
        let digestinfo = Hsm::create_digestinfo(payload.as_ref(), hash_algorithm)?;

        self.session.sign_pkcs1v1_5(signing_key, false, digestinfo)
    }

    pub fn put_rsa_key(&self, n: &BigUint, p: &BigUint, q: &BigUint) -> Result<u16, Error> {
        let algorithm = match n.bits() {
            1024 => bail!("YubiHSM does not support 1024-bit RSA keys."),
            2048 => Algorithm::Rsa2048,
            4096 => Algorithm::Rsa4096,
            b => bail!("Unknown RSA key size: {}", b),
        };

        let objects = self.session
            .list_objects()
            .object_type(ObjectType::Asymmetric)
            .execute()?;
        let object_id = match (1..).find(|id| objects.iter().find(|obj| obj.id == *id).is_none()) {
            Some(id) => id,
            None => bail!("Couldn't find a suitable free object ID"),
        };

        let this_authkey = self.session
            .get_object_info(self.authkey, ObjectType::AuthKey)?;

        self.session.put_key_rsa(
            object_id,
            "",
            &this_authkey.domains,
            &[Capability::AsymmetricSignPkcs],
            algorithm,
            &p.to_bytes_be(),
            &q.to_bytes_be(),
        )?;

        Ok(object_id)
    }

    pub fn logs(&self) -> Result<Vec<LogEntry>, Error> {
        Ok(self.session.get_logs()?
           .log_entries()
           .into_iter()
           .cloned()
           .collect::<Vec<_>>())
    }

    pub fn logs_since(&self, last_log_index: u16) -> Result<Vec<LogEntry>, Error> {
        let log = self.session.get_logs()?;
        let out = log.log_entries()
            .into_iter()
            .cloned()
            .filter(|l| l.index > last_log_index)
            .collect::<Vec<_>>();

        Ok(out)
    }

    pub fn set_log_index(&self, log_index: u16) -> Result<(), Error> {
        self.session.set_log_index(log_index)
    }
}
