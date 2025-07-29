pub mod errors;
pub mod issue_store;
pub mod repo;

pub use issue_store::IssueStore;

#[cfg(test)]
pub mod test_helpers {
    use super::repo::GitRepository;
    use crate::common::Identity;
    use std::path::Path;
    use tempfile::TempDir;

    /// Creates a temporary directory with an initialized Git repository and git-tracker
    pub fn setup_temp_repo() -> (TempDir, GitRepository) {
        let temp_dir = TempDir::new().expect("Failed to create temporary directory");
        let repo =
            GitRepository::init(temp_dir.path()).expect("Failed to initialize git repository");
        (temp_dir, repo)
    }

    /// Creates a consistent test identity for use in tests
    pub fn create_test_identity() -> Identity {
        Identity::new("Test User".to_string(), "test@example.com".to_string())
    }

    /// Verifies that a git object exists in the repository
    pub fn assert_git_object_exists(repo_path: &Path, oid: &gix::ObjectId) {
        let gix_repo = gix::open(repo_path).expect("Failed to open repository with gix");
        let _object = gix_repo
            .find_object(*oid)
            .expect("Git object should exist in repository");
    }

    /// Verifies that a git reference exists and points to the expected object
    pub fn assert_ref_exists(repo_path: &Path, ref_name: &str, expected_oid: &gix::ObjectId) {
        let gix_repo = gix::open(repo_path).expect("Failed to open repository with gix");
        let reference = gix_repo
            .find_reference(ref_name)
            .expect("Reference should exist");

        let target_ref = reference.target();
        let target = target_ref
            .try_id()
            .expect("Reference should point to an object ID");

        assert_eq!(
            target, *expected_oid,
            "Reference should point to expected object"
        );
    }
}
