use md5::{Digest, Md5};

/// Kugou MD5 signature for API params.
/// Sorts param list, concatenates with key, and returns MD5 hash.
/// platform: "android" (default) uses key 'OIlwieks28dk2k092lksi2UIkp'
/// platform: "web" uses key 'NVPh5oo715z5DIWAeQlhMDsWXXQV4hwt'
pub fn signature_params(params: &str, platform: &str, body: &str) -> String {
    let keyparam = if platform == "web" {
        "NVPh5oo715z5DIWAeQlhMDsWXXQV4hwt"
    } else {
        "OIlwieks28dk2k092lksi2UIkp"
    };

    let mut param_list: Vec<&str> = params.split('&').collect();
    param_list.sort();
    let sign_params = format!("{}{}{}{}", keyparam, param_list.join(""), body, keyparam);

    let mut hasher = Md5::new();
    hasher.update(sign_params.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_params() {
        let params = "appid=1005&clientver=11409&dfid=0&mid=123";
        let sig = signature_params(params, "android", "");
        assert!(!sig.is_empty());
        assert_eq!(sig.len(), 32); // MD5 hex is 32 chars
    }
}
