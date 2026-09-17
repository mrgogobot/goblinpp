# Goblin++ on GitHub: a first-time guide

Having a `.git` directory alone does not upload a project. If you have already
published this repository privately, skip the initial-upload steps below and
use the [release checklist](../RELEASE_CHECKLIST.md) for the remaining review.
Think of the terms this way: a **repository** is the project folder plus its
change history; a
**commit** is a saved snapshot; **publish/push** copies those snapshots to
GitHub; a **release** labels a reviewed version for others to download.

## The simplest route: GitHub Desktop

1. Review [the release checklist](../RELEASE_CHECKLIST.md), including the
   [software license](../LICENSE) and [documentation license](../LICENSE-DOCS.md),
   before making the project public. Confirm rights to license each included
   file and review the pending security and scientific gates.
2. Install [GitHub Desktop](https://desktop.github.com/) and sign in to the
   GitHub account you want to use. Signing into GitHub in a browser does not
   automatically sign GitHub Desktop in.
3. In Desktop, choose **File → Add Local Repository…** and select this
   `goblinpp` folder. If it reports that no repository exists, ask for help;
   do not create a second nested repository.
4. In the **Changes** tab, look through the pending files. Build outputs,
   personal runs, and large research data should not appear. Enter a summary
   such as `Prepare Goblin++ alpha source repository`, then **Commit to main**.
   This step stays on your computer.
5. When ready to upload, choose **Publish repository**. Confirm the account,
   repository name, and **Keep this code private** checkbox before clicking
   the final Publish button. Keeping it private is a reasonable first pass;
   do not turn it public until the checklist is complete.
6. On GitHub, open the **Actions** tab and inspect the Rust and editor-test
   results. A green result is a software check, not a claim of scientific
   validity. Fix failures before making a release.

Official guides: [add a local repository to GitHub Desktop](https://docs.github.com/en/desktop/adding-and-cloning-repositories/adding-a-repository-from-your-local-computer-to-github-desktop) and [publish an existing project](https://docs.github.com/en/desktop/adding-and-cloning-repositories/adding-an-existing-project-to-github-using-github-desktop).

## Later: public release and Zenodo

After the remaining review and passing checks, decide whether to make the
repository public. A GitHub **release** is created from a version tag; it is
not the same thing as a commit or an upload. Zenodo can archive a GitHub
release and assign a DOI, but that is a separate connection and review step.
Do not guess a DOI or put one in the citation file before Zenodo actually
assigns it.
