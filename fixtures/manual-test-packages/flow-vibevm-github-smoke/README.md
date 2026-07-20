# flow-vibevm-github-smoke

Throwaway test fixture for the GitHub API publish path.

The contents do nothing useful. The point is to have a real, valid vibevm
package whose name screams "test" so that when it lands in the public
`vibespecs` GitHub org, anyone reading the org page can tell at a glance
that the repo is not a production package and can be deleted at any time.

## How it gets used

```
vibe registry publish ./fixtures/manual-test-packages/flow-vibevm-github-smoke \
    --path <some-vibevm-project>
```

The default registry (`vibespecs` on GitHub) is used; the publish path
loads `~/.vibe/github.publish.token` (or `VIBEVM_PUBLISH_TOKEN_GITHUB`
/ legacy `VIBEVM_PUBLISH_TOKEN`), creates the repo on first publish via
the GitHub REST API, pushes the contents on `main`, and tags the
release.
