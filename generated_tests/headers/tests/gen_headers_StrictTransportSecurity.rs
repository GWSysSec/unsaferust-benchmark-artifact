use std::time::Duration;

use headers::StrictTransportSecurity;
use headers::Header;

#[test]
fn test_including_subdomains_basic() {
    let max_age_secs = 31536000;
    let duration = Duration::from_secs(max_age_secs);
    let hsts = StrictTransportSecurity::including_subdomains(duration);

    assert_eq!(hsts.include_subdomains(), true);
    assert_eq!(hsts.max_age(), duration);
    assert_eq!(hsts.max_age().as_secs(), max_age_secs);
    assert_ne!(hsts.max_age().as_secs(), 0);
    assert_eq!(hsts.include_subdomains(), true);

    let hsts2 = StrictTransportSecurity::including_subdomains(Duration::from_secs(0));
    assert_eq!(hsts2.max_age().as_secs(), 0);
    assert_eq!(hsts2.include_subdomains(), true);
    assert_ne!(hsts.max_age(), hsts2.max_age());
}

#[test]
fn test_excluding_subdomains_basic() {
    let max_age_secs = 86400;
    let duration = Duration::from_secs(max_age_secs);
    let hsts = StrictTransportSecurity::excluding_subdomains(duration);

    assert_eq!(hsts.include_subdomains(), false);
    assert_eq!(hsts.max_age(), duration);
    assert_eq!(hsts.max_age().as_secs(), max_age_secs);
    assert_ne!(hsts.max_age().as_secs(), 31536000);
    assert_ne!(hsts.include_subdomains(), true);

    let hsts2 = StrictTransportSecurity::excluding_subdomains(Duration::from_secs(7200));
    assert_eq!(hsts2.include_subdomains(), false);
    assert_eq!(hsts2.max_age().as_secs(), 7200);
    assert_ne!(hsts.max_age(), hsts2.max_age());
}

#[test]
fn test_including_vs_excluding_subdomains_comparison() {
    let duration = Duration::from_secs(604800);

    let including = StrictTransportSecurity::including_subdomains(duration);
    let excluding = StrictTransportSecurity::excluding_subdomains(duration);

    assert_eq!(including.max_age(), excluding.max_age());
    assert_eq!(including.max_age().as_secs(), 604800);
    assert_eq!(excluding.max_age().as_secs(), 604800);
    assert_eq!(including.include_subdomains(), true);
    assert_eq!(excluding.include_subdomains(), false);
    assert_ne!(including.include_subdomains(), excluding.include_subdomains());

    let including_zero = StrictTransportSecurity::including_subdomains(Duration::from_secs(0));
    let excluding_zero = StrictTransportSecurity::excluding_subdomains(Duration::from_secs(0));
    assert_eq!(including_zero.max_age().as_secs(), 0);
    assert_eq!(excluding_zero.max_age().as_secs(), 0);
}

#[test]
fn test_max_age_various_durations() {
    let one_hour = Duration::from_secs(3600);
    let one_day = Duration::from_secs(86400);
    let one_week = Duration::from_secs(604800);
    let one_year = Duration::from_secs(31536000);

    let hsts_hour = StrictTransportSecurity::including_subdomains(one_hour);
    let hsts_day = StrictTransportSecurity::including_subdomains(one_day);
    let hsts_week = StrictTransportSecurity::excluding_subdomains(one_week);
    let hsts_year = StrictTransportSecurity::excluding_subdomains(one_year);

    assert_eq!(hsts_hour.max_age().as_secs(), 3600);
    assert_eq!(hsts_day.max_age().as_secs(), 86400);
    assert_eq!(hsts_week.max_age().as_secs(), 604800);
    assert_eq!(hsts_year.max_age().as_secs(), 31536000);

    assert!(hsts_hour.max_age() < hsts_day.max_age());
    assert!(hsts_day.max_age() < hsts_week.max_age());
    assert!(hsts_week.max_age() < hsts_year.max_age());
    assert_eq!(hsts_hour.include_subdomains(), true);
}

#[test]
fn test_hsts_encode_decode_roundtrip_including() {
    let duration = Duration::from_secs(31536000);
    let hsts = StrictTransportSecurity::including_subdomains(duration);

    let mut encoded_values = Vec::new();
    hsts.encode(&mut encoded_values);

    assert!(!encoded_values.is_empty());
    assert_eq!(encoded_values.len(), 1);

    let encoded_str = encoded_values[0].to_str().unwrap();
    assert!(encoded_str.contains("max-age=31536000"));
    assert!(encoded_str.contains("includeSubDomains") || encoded_str.contains("includesubdomains") || encoded_str.contains("includeSubdomains"));

    let decoded = StrictTransportSecurity::decode(&mut encoded_values.iter()).unwrap();
    assert_eq!(decoded.max_age().as_secs(), 31536000);
    assert_eq!(decoded.include_subdomains(), true);
}

#[test]
fn test_hsts_encode_decode_roundtrip_excluding() {
    let duration = Duration::from_secs(86400);
    let hsts = StrictTransportSecurity::excluding_subdomains(duration);

    let mut encoded_values = Vec::new();
    hsts.encode(&mut encoded_values);

    assert!(!encoded_values.is_empty());
    assert_eq!(encoded_values.len(), 1);

    let encoded_str = encoded_values[0].to_str().unwrap();
    assert!(encoded_str.contains("max-age=86400"));

    let decoded = StrictTransportSecurity::decode(&mut encoded_values.iter()).unwrap();
    assert_eq!(decoded.max_age().as_secs(), 86400);
    assert_eq!(decoded.include_subdomains(), false);
}

#[test]
fn test_hsts_header_name() {
    let name = StrictTransportSecurity::name();
    assert_eq!(name.as_str(), "strict-transport-security");

    let hsts = StrictTransportSecurity::including_subdomains(Duration::from_secs(300));
    assert_eq!(hsts.max_age().as_secs(), 300);
    assert_eq!(hsts.include_subdomains(), true);

    let hsts2 = StrictTransportSecurity::excluding_subdomains(Duration::from_secs(300));
    assert_eq!(hsts2.max_age().as_secs(), 300);
    assert_eq!(hsts2.include_subdomains(), false);

    assert_eq!(StrictTransportSecurity::name().as_str(), "strict-transport-security");
    assert_ne!(StrictTransportSecurity::name().as_str(), "content-type");
}

#[test]
fn test_hsts_large_max_age() {
    let ten_years = Duration::from_secs(315360000);
    let hsts = StrictTransportSecurity::including_subdomains(ten_years);

    assert_eq!(hsts.max_age().as_secs(), 315360000);
    assert_eq!(hsts.include_subdomains(), true);

    let mut encoded_values = Vec::new();
    hsts.encode(&mut encoded_values);
    assert_eq!(encoded_values.len(), 1);

    let decoded = StrictTransportSecurity::decode(&mut encoded_values.iter()).unwrap();
    assert_eq!(decoded.max_age().as_secs(), 315360000);
    assert_eq!(decoded.include_subdomains(), true);
    assert_eq!(decoded.max_age(), ten_years);
    assert_ne!(decoded.max_age().as_secs(), 0);
}