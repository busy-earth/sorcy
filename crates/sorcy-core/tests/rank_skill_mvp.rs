use std::fs;

use sorcy_core::model::Ecosystem;
use sorcy_core::{
    classify_seeded_tier, install_sorcy_rank_skill_from_source, parse_rank_overrides,
    read_rank_overrides, RelevanceTier, RANK_OVERRIDES_FILE_NAME, SKILL_INSTRUCTIONS_FILE_NAME,
    SKILL_RANKINGS_FILE_NAME,
};

#[test]
fn low_value_seed_list_classifies_known_candidates() {
    assert_eq!(
        classify_seeded_tier(&Ecosystem::Cargo, "libc"),
        Some(RelevanceTier::Distant)
    );
    assert_eq!(
        classify_seeded_tier(&Ecosystem::Cargo, "lazy_static"),
        Some(RelevanceTier::Void)
    );
    assert_eq!(
        classify_seeded_tier(&Ecosystem::Npm, "left-pad"),
        Some(RelevanceTier::Void)
    );
    assert_eq!(
        classify_seeded_tier(&Ecosystem::Python, "typing-extensions"),
        Some(RelevanceTier::Distant)
    );
    assert_eq!(classify_seeded_tier(&Ecosystem::Cargo, "tokio"), None);
}

#[test]
fn rank_overrides_file_parses_expected_tiers() {
    let overrides = parse_rank_overrides(
        r#"
[tiers]
tokio = "Orbit"
libc = "Void"
"#,
    )
    .expect("parse sorcy-rank.toml");

    assert_eq!(overrides.tier_for("tokio"), Some(RelevanceTier::Orbit));
    assert_eq!(overrides.tier_for("Tokio"), Some(RelevanceTier::Orbit));
    assert_eq!(overrides.tier_for("libc"), Some(RelevanceTier::Void));
    assert_eq!(overrides.tier_for("serde"), None);
}

#[test]
fn read_rank_overrides_from_project_root() {
    let temp = tempfile::tempdir().expect("temp dir");
    fs::write(
        temp.path().join(RANK_OVERRIDES_FILE_NAME),
        r#"
[tiers]
serde = "Transit"
"#,
    )
    .expect("write overrides");

    let overrides = read_rank_overrides(temp.path())
        .expect("read rank overrides")
        .expect("overrides exists");
    assert_eq!(overrides.tier_for("serde"), Some(RelevanceTier::Transit));
}

#[test]
fn install_skill_copies_files_and_preserves_existing_rankings() {
    let temp = tempfile::tempdir().expect("temp dir");
    let source_dir = temp.path().join("source-skill");
    let install_root = temp.path().join(".skills");
    fs::create_dir_all(&source_dir).expect("create source dir");
    fs::write(
        source_dir.join(SKILL_INSTRUCTIONS_FILE_NAME),
        "source instructions v1\n",
    )
    .expect("write source skill");
    fs::write(
        source_dir.join(SKILL_RANKINGS_FILE_NAME),
        "# Sorcy Rankings\nsource placeholder\n",
    )
    .expect("write source rankings");

    let installed = install_sorcy_rank_skill_from_source(&source_dir, &install_root)
        .expect("install skill first run");
    let installed_skill_file = installed.target_dir.join(SKILL_INSTRUCTIONS_FILE_NAME);
    let installed_rankings_file = installed.target_dir.join(SKILL_RANKINGS_FILE_NAME);
    assert_eq!(
        fs::read_to_string(&installed_skill_file).expect("read installed skill"),
        "source instructions v1\n"
    );
    assert_eq!(
        fs::read_to_string(&installed_rankings_file).expect("read installed rankings"),
        "# Sorcy Rankings\nsource placeholder\n"
    );

    fs::write(
        &installed_rankings_file,
        "# Sorcy Rankings\ncustom ranking data\n",
    )
    .expect("write custom rankings");
    fs::write(
        source_dir.join(SKILL_INSTRUCTIONS_FILE_NAME),
        "source instructions v2\n",
    )
    .expect("update source skill");

    let reinstalled = install_sorcy_rank_skill_from_source(&source_dir, &install_root)
        .expect("install skill second run");
    assert_eq!(
        fs::read_to_string(reinstalled.target_dir.join(SKILL_INSTRUCTIONS_FILE_NAME))
            .expect("read installed skill after reinstall"),
        "source instructions v2\n"
    );
    assert_eq!(
        fs::read_to_string(reinstalled.target_dir.join(SKILL_RANKINGS_FILE_NAME))
            .expect("read installed rankings after reinstall"),
        "# Sorcy Rankings\ncustom ranking data\n"
    );
}
