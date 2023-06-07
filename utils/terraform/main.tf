# Mangement of the GitHub project.

resource "github_repository" "viguno" {
  name        = "viguno"
  description = "Versatile Interface for Genetics Utilization of Nice Ontologies"

  has_issues = true
  visibility = "public"

  allow_auto_merge       = true
  allow_merge_commit     = false
  allow_rebase_merge     = false
  has_downloads          = true
  delete_branch_on_merge = true

  squash_merge_commit_message = "BLANK"
  squash_merge_commit_title   = "PR_TITLE"

  template {
    owner                = "bihealth"
    repository           = "tpl-rs"
    include_all_branches = true
  }
}
