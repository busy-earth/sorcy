use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use sorcy_core::model::Ecosystem;
use sorcy_core::{
    classify_seeded_tier, install_sorcy_rank_skill_from_source,
    install_sorcy_rank_skill_with_root_override, parse_rank_overrides, read_rank_overrides,
    RelevanceTier, SkillInstallScope, PROJECT_SKILLS_DIR, RANK_OVERRIDES_FILE_NAME,
    SKILLS_DIR_OVERRIDE_ENV, SKILL_INSTRUCTIONS_FILE_NAME, SKILL_RANKINGS_FILE_NAME,
    SORCY_RANK_SKILL_NAME,
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

#[test]
fn project_local_install_defaults_to_claude_skills_path() {
    let temp = tempfile::tempdir().expect("temp dir");
    let source_root = repo_skill_source_root();
    let installed = with_skills_dir_override(source_root.as_path(), || {
        install_sorcy_rank_skill_with_root_override(
            temp.path(),
            SkillInstallScope::ProjectLocal,
            None,
        )
    })
    .expect("install skill");

    let expected = temp
        .path()
        .join(PROJECT_SKILLS_DIR)
        .join(SORCY_RANK_SKILL_NAME);
    assert_eq!(installed.target_dir, expected);
    assert!(installed
        .target_dir
        .join(SKILL_INSTRUCTIONS_FILE_NAME)
        .is_file());
    assert!(installed
        .target_dir
        .join(SKILL_RANKINGS_FILE_NAME)
        .is_file());
}

fn repo_skill_source_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../skills")
        .to_path_buf()
}

fn with_skills_dir_override<T>(
    skills_root: &Path,
    run: impl FnOnce() -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    let env_lock = env_lock();
    let _guard = env_lock.lock().expect("env mutex poisoned");

    let previous = std::env::var_os(SKILLS_DIR_OVERRIDE_ENV);
    unsafe {
        std::env::set_var(SKILLS_DIR_OVERRIDE_ENV, skills_root);
    }
    let result = run();
    match previous {
        Some(value) => unsafe {
            std::env::set_var(SKILLS_DIR_OVERRIDE_ENV, value);
        },
        None => unsafe {
            std::env::remove_var(SKILLS_DIR_OVERRIDE_ENV);
        },
    }
    result
}

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}
