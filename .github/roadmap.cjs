const childProcess = require('node:child_process')

function judge(command, query) {
  const input = JSON.stringify(query)
  const inputBytes = Buffer.byteLength(input)
  // Selection can echo the release tag and its version, and adds two indices per title.
  const maxBuffer = query.operation === 'release-selection'
    ? 3 * inputBytes + 64 * query.milestones.length + 1024
    : inputBytes + 1024
  const bytes = childProcess.execFileSync('chrono-host-release', [command], {
    input, encoding: 'utf8', stdio: ['pipe', 'pipe', 'inherit'],
    maxBuffer,
  })
  return JSON.parse(bytes)
}

const roadmap = query => judge('roadmap-v1', query)

module.exports = async function syncRoadmap(github, context, plan) {
  const rolling = roadmap({operation: 'rolling-plan', plan})
  const repo = context.repo
  const issues = github.rest.issues
  const all = (method, parameters) => github.paginate(method, {...repo, per_page: 100, ...parameters})
  const releases = await all(github.rest.repos.listReleases, {})
  const milestones = await all(issues.listMilestones, {state: 'all'})

  async function ensure(title) {
    const existing = milestones.find(m => m.title === title)
    if (existing) return existing
    const {data} = await issues.createMilestone({...repo, title})
    milestones.push(data)
    return data
  }
  const releasesForPlan = releases.map(({tag_name, draft, prerelease, published_at}) =>
    ({tag_name, draft, prerelease, published_at}))
  let target = plan ? 'v' + plan.version : null
  if (rolling) {
    const latest = judge('template-latest-stable-v1', releasesForPlan)
    target = 'v' + judge('template-next-version-v1', {
      floor: plan.version, latest: latest === null ? null : latest.slice(1),
    })
  }
  const releaseEvent = context.payload.release?.tag_name
  const selection = roadmap({
    operation: 'release-selection', reconcile: true, releases: releasesForPlan,
    event_tag: releaseEvent, target, milestones: milestones.map(m => m.title),
  })
  const unassigned = await all(issues.listForRepo, {state: 'open', milestone: 'none'})
  const numbers = roadmap({operation: 'unassigned',
    issues: unassigned.map(({number, state, pull_request, milestone}) => ({number, state, pull_request, milestone}))})
  for (const number of numbers) {
    const milestone = await ensure(selection.destination)
    await issues.update({...repo, issue_number: number, milestone: milestone.number})
  }
  // Publishers using GITHUB_TOKEN wake this workflow via workflow_run.
  // Use the newest public release even when this host has no version plan.
  for (const title of selection.ensure) await ensure(title)

  for (const known of milestones) {
    if (!Object.hasOwn(selection.refresh, known.title)) continue
    const indices = selection.refresh[known.title]
    const final = releases[indices.final]
    const preview = releases[indices.preview]
    const {data: milestone} = await issues.getMilestone({...repo, milestone_number: known.number})
    const lines = []
    if (preview) lines.push(`测试版 [${preview.tag_name}](${preview.html_url})`)
    if (final) lines.push(`正式版 [${final.tag_name}](${final.html_url})`)
    const update = roadmap({operation: 'milestone-update', milestone: milestone.number,
      original: milestone.description || '', lines, state: milestone.state,
      final_release: Boolean(final), open_issues: milestone.open_issues})
    if (update !== null) {
      await issues.updateMilestone({...repo, milestone_number: known.number, ...update})
    }
  }
}

// A workflow_run identifies the original release run across retry attempts.
module.exports.releaseIssue = async function releaseIssue(github, context) {
  const repo = context.repo
  const runId = context.payload.workflow_run.id
  const {data: run} = await github.rest.actions.getWorkflowRun({...repo, run_id: runId})
  const result = {status: run.status, conclusion: run.conclusion}
  if (!roadmap({operation: 'release-run', ...result})) return
  const issues = github.rest.issues
  const label = 'release-failure'
  const marker = `<!-- chrono-harness release-run ${runId} -->`
  const history = await github.paginate(issues.listForRepo, {...repo, labels: label, state: 'all', per_page: 100})
  const {action, issue} = roadmap({operation: 'release-issue', ...result, run_id: runId,
    issues: history.map(({number, state, state_reason, pull_request, user, body}) =>
      ({number, state, state_reason, pull_request, user: user && {login: user.login}, body}))})
  switch (action) {
    case 'none':
      return
    case 'close': {
      const original = history.find(item => item.number === issue).body
      const recovery = `同次运行已恢复。[成功的第 ${run.run_attempt} 次尝试](${run.html_url}/attempts/${run.run_attempt})`
      const body = roadmap({operation: 'recovery-body', original, recovery})
      await issues.update({...repo, issue_number: issue, state: 'closed', body})
      return
    }
    case 'reopen':
      await issues.update({...repo, issue_number: issue, state: 'open'})
      return
    case 'create':
      break
    default:
      throw new Error('Unknown roadmap issue action: ' + action)
  }
  try {
    await issues.getLabel({...repo, name: label})
  } catch (error) {
    if (error.status !== 404) throw error
    try {
      await issues.createLabel({...repo, name: label, color: 'b60205', description: 'Failed product release run'})
    } catch (conflict) {
      if (conflict.status !== 422) throw conflict
      await issues.getLabel({...repo, name: label})
    }
  }
  await issues.create({...repo, labels: [label], title: `发行失败 · ${run.name} #${run.run_number}`,
    body: `${marker}\n\n工作流在第 ${run.run_attempt} 次尝试返回 \`${run.conclusion}\`。提交为 \`${run.head_sha}\`。同一次运行重试成功会关闭本事项。通过后续版本修复时，从修复事项或 PR 引用此处并保留原始运行。\n\n[查看发布作业](${run.html_url}/attempts/${run.run_attempt})`})
}
