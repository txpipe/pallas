#[cfg(test)]
mod test {

    use assert_cmd::Command;
    use predicates::prelude::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    use crate::kes::common::generate_crypto_secure_seed;

    const PRG: &str = env!("CARGO_PKG_NAME");

    #[test]
    fn correct_output_help_arg() {
        let mut cmd = Command::cargo_bin(PRG).unwrap();
        cmd.arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("Usage"));
    }

    #[test]
    fn correct_output_version_arg() {
        let mut cmd = Command::cargo_bin(PRG).unwrap();
        let ver = PRG.to_owned() + " " + env!("CARGO_PKG_VERSION");
        cmd.arg("--version")
            .assert()
            .success()
            .stdout(predicate::str::contains(ver));
    }

    fn hex_length_check(option: &str, len: i32) {
        let mut cmd = Command::cargo_bin(PRG).unwrap();
        let regex = format!("^[0-9a-f]{{{}}}$", 2 * len);
        let is_len_byte_hex = predicate::str::is_match(regex).unwrap();
        cmd.arg(option).assert().success().stdout(is_len_byte_hex);
    }

    #[test]
    fn correct_length_output_generate_seed() {
        hex_length_check("generate-seed", 32)
    }

    #[test]
    fn correct_length_output_generate_signing_key() {
        hex_length_check("generate-sk", 612)
    }

    #[test]
    fn deriving_sk_from_seed_is_deterministic() {
        let mut random_bytes = [0u8; 32];
        let _ = generate_crypto_secure_seed(&mut random_bytes[..]);
        let mut seed = NamedTempFile::new().unwrap();
        write!(seed, "{}", hex::encode(&random_bytes)).unwrap();
        let seed_file_name = (*seed.path()).display().to_string();

        let sk1 = Command::cargo_bin(PRG)
            .unwrap()
            .args(["derive-sk", "-f", &seed_file_name])
            .assert()
            .get_output()
            .stdout
            .clone();
        let sk2 = Command::cargo_bin(PRG)
            .unwrap()
            .args(["derive-sk", "-f", &seed_file_name])
            .assert()
            .get_output()
            .stdout
            .clone();
        let sk3 = Command::cargo_bin(PRG)
            .unwrap()
            .write_stdin(hex::encode(&random_bytes))
            .args(["derive-sk", "-f", "-"])
            .assert()
            .get_output()
            .stdout
            .clone();
        // derivation is deterministic is file is used
        assert_eq!(sk1, sk2);
        // derivation is deterministic irrespective of file/stdin input
        assert_eq!(sk1, sk3);
        // output is 1224 character hex, meaning 612-byte payload
        assert_eq!(sk1.len(), 1224)
    }

    #[test]
    fn deriving_pk_from_sk_is_deterministic() {
        let mut random_bytes = [0u8; 612];
        let _ = generate_crypto_secure_seed(&mut random_bytes[..]);
        let mut sk = NamedTempFile::new().unwrap();
        write!(sk, "{}", hex::encode(&random_bytes)).unwrap();
        let sk_file_name = (*sk.path()).display().to_string();

        let pk1 = Command::cargo_bin(PRG)
            .unwrap()
            .args(["derive-pk", "-f", &sk_file_name])
            .assert()
            .get_output()
            .stdout
            .clone();
        let pk2 = Command::cargo_bin(PRG)
            .unwrap()
            .args(["derive-pk", "-f", &sk_file_name])
            .assert()
            .get_output()
            .stdout
            .clone();
        let pk3 = Command::cargo_bin(PRG)
            .unwrap()
            .write_stdin(hex::encode(&random_bytes))
            .args(["derive-pk", "-f", "-"])
            .assert()
            .get_output()
            .stdout
            .clone();
        // derivation is deterministic if file input is used
        assert_eq!(pk1, pk2);
        // derivation is deterministic irrespective of file/stdin input
        assert_eq!(pk1, pk3);
        // output is 64 character hex, meaning 32-byte payload
        assert_eq!(pk1.len(), 64)
    }

    #[test]
    fn get_period_from_sk_is_zero_in_the_beginning() {
        let mut random_bytes = [0u8; 32];
        let _ = generate_crypto_secure_seed(&mut random_bytes[..]);
        let mut seed = NamedTempFile::new().unwrap();
        write!(seed, "{}", hex::encode(&random_bytes)).unwrap();
        let seed_file_name = (*seed.path()).display().to_string();
        let is_zero = predicate::str::is_match("^0$").unwrap();

        let sk = Command::cargo_bin(PRG)
            .unwrap()
            .args(["derive-sk", "-f", &seed_file_name])
            .assert()
            .get_output()
            .stdout
            .clone();

        Command::cargo_bin(PRG)
            .unwrap()
            .write_stdin(sk)
            .args(["period", "-f", "-"])
            .assert()
            .success()
            .stdout(is_zero);
    }

    #[test]
    fn sign_message_and_verify_the_resultant_signature() {
        let mut random_bytes = [0u8; 32];
        let _ = generate_crypto_secure_seed(&mut random_bytes[..]);
        let mut seed_file = NamedTempFile::new().unwrap();
        write!(seed_file, "{}", hex::encode(&random_bytes)).unwrap();
        let seed_file_name = (*seed_file.path()).display().to_string();

        let msg = String::from("the message to be encrypted").into_bytes();

        let sk = Command::cargo_bin(PRG)
            .unwrap()
            .args(["derive-sk", "-f", &seed_file_name])
            .assert()
            .get_output()
            .stdout
            .clone();

        let mut sk_file = NamedTempFile::new().unwrap();
        let sk_str = String::from_utf8(sk.clone()).expect("should be bytes from sk");
        write!(sk_file, "{}", sk_str).unwrap();
        let sk_file_name = (*sk_file.path()).display().to_string();

        let pk = Command::cargo_bin(PRG)
            .unwrap()
            .write_stdin(sk)
            .args(["derive-pk", "-f", "-"])
            .assert()
            .get_output()
            .stdout
            .clone();

        let mut pk_file = NamedTempFile::new().unwrap();
        let pk_str = String::from_utf8(pk.clone()).expect("should be bytes from pk");
        write!(pk_file, "{}", pk_str).unwrap();
        let pk_file_name = (*pk_file.path()).display().to_string();

        let sig = Command::cargo_bin(PRG)
            .unwrap()
            .write_stdin(msg.clone())
            .args(["sign", "-f", &sk_file_name])
            .assert()
            .get_output()
            .stdout
            .clone();

        let printed_ok = predicate::str::is_match("^OK\n$").unwrap();
        let sig_str = String::from_utf8(sig.clone()).expect("should be bytes from sig");
        let mut sig_file = NamedTempFile::new().unwrap();
        write!(sig_file, "{}", sig_str).unwrap();
        let sig_file_name = (*sig_file.path()).display().to_string();

        Command::cargo_bin(PRG)
            .unwrap()
            .write_stdin(msg.clone())
            .args([
                "verify",
                "-s",
                &sig_file_name,
                "-f",
                &pk_file_name,
                "-p",
                "0",
            ])
            .assert()
            .success()
            .stdout(printed_ok.clone());

        let sk1 = Command::cargo_bin(PRG)
            .unwrap()
            .args(["update", "-f", &sk_file_name])
            .assert()
            .get_output()
            .stdout
            .clone();

        let mut sk1_file = NamedTempFile::new().unwrap();
        let sk1_str = String::from_utf8(sk1.clone()).expect("should be bytes from sk");
        write!(sk1_file, "{}", sk1_str).unwrap();
        let sk1_file_name = (*sk1_file.path()).display().to_string();

        let sig1 = Command::cargo_bin(PRG)
            .unwrap()
            .write_stdin(msg.clone())
            .args(["sign", "-f", &sk1_file_name])
            .assert()
            .get_output()
            .stdout
            .clone();

        let printed_fail = predicate::str::is_match("^Fail\n$").unwrap();
        let sig1_str = String::from_utf8(sig1.clone()).expect("should be bytes from sig");
        let mut sig1_file = NamedTempFile::new().unwrap();
        write!(sig1_file, "{}", sig1_str).unwrap();
        let sig1_file_name = (*sig1_file.path()).display().to_string();

        Command::cargo_bin(PRG)
            .unwrap()
            .write_stdin(msg.clone())
            .args([
                "verify",
                "-s",
                &sig1_file_name,
                "-f",
                &pk_file_name,
                "-p",
                "0",
            ])
            .assert()
            .success()
            .stdout(printed_fail);

        Command::cargo_bin(PRG)
            .unwrap()
            .write_stdin(msg)
            .args([
                "verify",
                "-s",
                &sig1_file_name,
                "-f",
                &pk_file_name,
                "-p",
                "1",
            ])
            .assert()
            .success()
            .stdout(printed_ok);
    }
}
