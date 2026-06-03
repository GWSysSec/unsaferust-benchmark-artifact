use dashmap::DashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
struct UserProfile {
    display_name: String,
    email: Option<String>,
    tags: Vec<String>,
}

#[test]
fn ref_map_projects_nested_fields_without_cloning_whole_value() {
    let profiles: DashMap<String, UserProfile> = DashMap::new();

    assert!(profiles.is_empty());
    assert_eq!(
        profiles.insert(
            "alice".to_string(),
            UserProfile {
                display_name: "Alice A.".to_string(),
                email: Some("alice@example.com".to_string()),
                tags: vec!["admin".to_string(), "team-blue".to_string()],
            },
        ),
        None
    );
    assert_eq!(
        profiles.insert(
            "bob".to_string(),
            UserProfile {
                display_name: "Bobby".to_string(),
                email: None,
                tags: vec!["guest".to_string()],
            },
        ),
        None
    );

    assert_eq!(profiles.len(), 2);
    assert!(profiles.contains_key("alice"));
    assert!(profiles.contains_key("bob"));

    {
        let alice_tags = profiles
            .get("alice")
            .expect("alice should be present")
            .map(|profile| &profile.tags);

        assert_eq!(alice_tags.len(), 2);
        assert!(alice_tags.iter().any(|tag| tag == "admin"));
        assert!(alice_tags.iter().any(|tag| tag == "team-blue"));
        assert_eq!(alice_tags.join(","), "admin,team-blue");
    }

    {
        let bob_name = profiles
            .get("bob")
            .expect("bob should be present")
            .map(|profile| &profile.display_name);

        assert_eq!(&*bob_name, "Bobby");
        assert_eq!(bob_name.len(), 5);
    }

    assert_eq!(
        profiles.insert(
            "carol".to_string(),
            UserProfile {
                display_name: "Carol".to_string(),
                email: Some("carol@example.com".to_string()),
                tags: Vec::new(),
            },
        ),
        None
    );
    assert_eq!(profiles.len(), 3);
}

#[test]
fn ref_try_map_successfully_projects_optional_field_and_allows_later_updates() {
    let profiles: DashMap<String, UserProfile> = DashMap::new();

    profiles.insert(
        "alice".to_string(),
        UserProfile {
            display_name: "Alice".to_string(),
            email: Some("alice@example.com".to_string()),
            tags: vec!["verified".to_string()],
        },
    );

    let projected_email = profiles
        .get("alice")
        .expect("alice should be present")
        .try_map(|profile| profile.email.as_ref());

    assert!(projected_email.is_ok());

    {
        let email = projected_email.expect("email should be projected");
        assert_eq!(&*email, "alice@example.com");
        assert!(email.ends_with("@example.com"));
        assert_eq!(email.split('@').next(), Some("alice"));
    }

    {
        let mut alice = profiles
            .get_mut("alice")
            .expect("alice should still be mutable after mapped ref is dropped");
        alice.tags.push("paid".to_string());
        alice.email = Some("alice@new.example".to_string());
    }

    let updated = profiles
        .get("alice")
        .expect("alice should remain present")
        .try_map(|profile| profile.email.as_ref())
        .expect("updated email should still be present");

    assert_eq!(&*updated, "alice@new.example");
}

#[test]
fn ref_try_map_failure_returns_original_ref_for_fallback_inspection() {
    let profiles: DashMap<String, UserProfile> = DashMap::new();

    profiles.insert(
        "bob".to_string(),
        UserProfile {
            display_name: "Bob".to_string(),
            email: None,
            tags: vec!["guest".to_string(), "trial".to_string()],
        },
    );

    let projected_email = profiles
        .get("bob")
        .expect("bob should be present")
        .try_map(|profile| profile.email.as_ref());

    assert!(projected_email.is_err());

    let original_ref = projected_email.expect_err("missing email should return original ref");
    assert_eq!(original_ref.display_name, "Bob");
    assert!(original_ref.email.is_none());
    assert_eq!(original_ref.tags.len(), 2);
    assert!(original_ref.tags.iter().any(|tag| tag == "guest"));

    let projected_first_tag = original_ref
        .try_map(|profile| profile.tags.first())
        .expect("bob should have a first tag");

    assert_eq!(&*projected_first_tag, "guest");
}