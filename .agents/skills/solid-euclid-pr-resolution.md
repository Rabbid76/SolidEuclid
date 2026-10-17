# SolidEuclid PR Review Resolution Skill

This skill defines the strict process for resolving a pull request's code review: verifying
suggested changes, applying them, replying to every review comment, and resolving the threads.
Load it whenever the task is to "resolve a PR", "address review comments", "handle PR feedback", or
"respond to a reviewer review".

## Hard Rules

- **NEVER merge the PR.** Not with `gh pr merge`, not by enabling auto-merge. Merging is always a
  human decision — finish the review resolution and stop.
- **Never rewrite history.** No `git commit --amend`, no `git push --force`. Every review fix is a
  **new** conventional commit (`type: lowercase description`, no ticket numbers).
- **Never silently ignore or resolve a comment.** Every handled thread gets a reply comment first,
  then is marked resolved.
- **Verify before implementing.** A reviewer suggestion is a hypothesis, not an instruction. Confirm
  it against the actual code before changing anything.

## Workflow

### 1. Identify the PR and check out its branch

Always pass the PR number explicitly — do not rely on the currently checked-out branch:

```bash
gh pr view <PR_NUMBER> --json number,title,headRefName,url
gh pr checkout <PR_NUMBER>
git pull
```

### 2. Fetch the unresolved review threads

Thread resolution state is only available via GraphQL (the REST API does not expose it):

```bash
gh api graphql -f query='
query($owner: String!, $repo: String!, $pr: Int!, $after: String) {
  repository(owner: $owner, name: $repo) {
    pullRequest(number: $pr) {
      reviewThreads(first: 100, after: $after) {
        pageInfo { hasNextPage endCursor }
        nodes {
          id
          isResolved
          isOutdated
          path
          line
          comments(first: 50) {
            nodes { databaseId author { login } body }
          }
        }
      }
    }
  }
}' -F owner=Rabbid76 -F repo=SolidEuclid -F pr=<PR_NUMBER>
```

- `id` (`PRRT_...`) is the **thread id** — needed to resolve the thread.
- `databaseId` (numeric) is the **comment id** — needed to reply via REST.
- If `pageInfo.hasNextPage` is `true`, repeat the query with `-F after=<endCursor>` until all
  thread pages are fetched — otherwise threads beyond the first 100 would be silently missed.
- Only threads with `isResolved: false` are in scope. Already-resolved threads are not
  reconsidered unless the user explicitly asks or the thread was reopened.
- `isOutdated: true` threads still need a reply and resolution.

### 3. Verify each suggestion

For every unresolved thread, before writing any code:

- Read the referenced code (`path` / `line`) and understand what the comment claims.
- Check whether the claim is actually true in the current code — reviewers comment on stale
  diffs, misread context, or suggest changes that conflict with project conventions.
- Classify the suggestion:
  - **Valid** → implement it.
  - **Valid in intent, wrong in form** → implement a different solution that addresses the
    underlying concern.
  - **Invalid** → do not change the code; prepare a reasoned reply instead.

### 4. Implement and validate

- Apply the accepted changes following the repository coding conventions (see
  [AGENTS.md](../../AGENTS.md)).
- Run the test suites the change touches: `cargo test` for the core, `npm test` for the web API,
  `npm run test:cpp` for the C API.
- Commit as one or more new conventional commits and push:

```bash
git add <files>
git commit -m "fix: lowercase description of the review fix"
git push
```

### 5. Reply to every handled comment

Reply via REST using the **numeric** `databaseId` of the thread's first comment:

```bash
gh api repos/Rabbid76/SolidEuclid/pulls/<PR_NUMBER>/comments/<COMMENT_DATABASE_ID>/replies -f body="<reply text>"
```

Reply content rules:

- **Applied:** state that the suggestion was applied and name the commit, e.g. `Applied in
<short-sha>.` Add a sentence on what was changed if it is not obvious.
- **Applied differently:** explain how the implementation differs from the suggestion and why.
- **Not applied:** give a clear, professional explanation of why the current code is preferred.
  Include general considerations (conventions, trade-offs, invariants) where they help the
  reviewer follow the reasoning.
- One reply per thread; when several threads share one root cause, fix it once but still reply to
  each thread individually.

### 6. Resolve each handled thread

After the reply is posted, resolve the thread via GraphQL using the `PRRT_...` thread id:

```bash
gh api graphql -f query='
mutation($threadId: ID!) {
  resolveReviewThread(input: {threadId: $threadId}) {
    thread { isResolved }
  }
}' -F threadId=<THREAD_ID>
```

### 7. Final verification

Re-run the thread query from step 2 and confirm every thread that was in scope is now
`isResolved: true`. The task is **not complete** until each handled thread has both a reply and a
resolution. If the available tools cannot post a reply or resolve a thread, state that explicitly
and provide the exact reply texts and resolution statuses that are still pending.

Then stop. **Do not merge the PR.**

## Completion Checklist

- [ ] Every unresolved thread was verified against the actual code.
- [ ] Confirmed-valid suggestions are implemented, built, tested, and formatted.
- [ ] Fixes are pushed as new commits (no amend, no force push).
- [ ] Every handled thread has a reply stating applied / applied differently / not applied, with
      reasoning.
- [ ] Every handled thread is marked resolved.
- [ ] The PR was **not** merged.
