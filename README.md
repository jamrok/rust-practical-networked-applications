# 🦀 Practical Networked Applications (PNA) in Rust

[![Code Checks][badge_gha_checks]][gha_checks]
[![Security Audit][badge_gha_audit]][gha_audit]

A training course about practical systems software construction in Rust.

- This is my implementation of Projects 1 to 4 from the [PingCAP Talent Plan][pna_talent_plan] PNA Course:
  - [TP 201: Practical Networked Applications in Rust][pna_tp201]. A series of projects that incrementally develop a single Rust project from the ground up into a high-performance, networked, parallel and asynchronous key/value store. Along the way various real-world Rust development subject matter are explored and discussed.
- It was also a good use-case to deep dive into GitHub Actions and see how it compares to GitLab CI.

### 📑 Project outline:

- [Project 1: The Rust toolbox][project_1]
- [Project 2: Log-structured file I/O][project_2]
- [Project 3: Synchronous client-server networking][project_3]
- [Project 4: Concurrency and Parallelism][project_4]

### 🪝 Git Hooks
[Git Hooks][git_hooks] are in the [.hooks](.hooks) directory.

Run [`.hooks/enable`](.hooks/enable) or [`.hooks/disable`](.hooks/disable) to enable or disable them respectively.

The main hook is [`.hooks/pre-commit`](.hooks/pre-commit):
- It is a script that is triggered by the `git commit` command.
- It runs various commands in the script (similar to what is run in CI) to verify the files before completing the `git commit` command.
  - ℹ️ Some commands in the script check the files in the working directory, not only the files staged for commit.
  - ℹ️ Ensure everything you want to commit is staged as they will be committed if the checks pass.
- To skip triggering this hook, append `-n` or `--no-verify` to the `git commit` command you ran.

[badge_gha_audit]: https://github.com/jamrok/rust-practical-networked-applications/actions/workflows/audit.yml/badge.svg
[badge_gha_checks]: https://github.com/jamrok/rust-practical-networked-applications/actions/workflows/checks.yml/badge.svg
[gha_audit]: https://github.com/jamrok/rust-practical-networked-applications/actions/workflows/audit.yml
[gha_checks]: https://github.com/jamrok/rust-practical-networked-applications/actions/workflows/checks.yml
[git_hooks]: https://git-scm.com/docs/githooks
[pna_talent_plan]: https://github.com/pingcap/talent-plan
[pna_tp201]: https://github.com/pingcap/talent-plan/blob/master/courses/rust/docs/lesson-plan.md
[project_1]: https://github.com/pingcap/talent-plan/blob/master/courses/rust/projects/project-1
[project_2]: https://github.com/pingcap/talent-plan/blob/master/courses/rust/projects/project-2
[project_3]: https://github.com/pingcap/talent-plan/blob/master/courses/rust/projects/project-3
[project_4]: https://github.com/pingcap/talent-plan/blob/master/courses/rust/projects/project-4
