# Merge-based tracking fork of rtk-ai/rtk

Upstream rtk-ai/rtk is active but slow: 100+ open community PRs with good fixes going unreviewed. This fork (kylehgc/rtk) stays based on upstream `develop` and periodically pulls it in with `git merge upstream/develop` — never rebasing published history — while cherry-picking curated community PRs and original fixes as regular commits on top.

Merge-based (not a rebase overlay) because the fork accepts external contributions: rebasing would rewrite history and break every branch based on it. Not a divergent fork because upstream still merges fixes we want for free; divergence remains an escape hatch if upstream dies.
