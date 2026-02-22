const { Anthropic } = require('@anthropic-ai/sdk');
const fs = require('fs');

async function analyzeIssue() {
  const ANTHROPIC_API_KEY = process.env.ANTHROPIC_API_KEY;
  const ISSUE_TITLE = process.env.ISSUE_TITLE || '';
  const ISSUE_BODY = process.env.ISSUE_BODY || '';
  const ISSUE_NUMBER = process.env.ISSUE_NUMBER;
  const OWNER = process.env.OWNER;
  const REPO = process.env.REPO;

  if (!ANTHROPIC_API_KEY) {
    console.log('ANTHROPIC_API_KEY not set, skipping triage');
    setOutput('labels', '');
    setOutput('needs_repro', 'false');
    return;
  }

  if (!ISSUE_TITLE) {
    console.log('No issue title, skipping');
    setOutput('labels', '');
    setOutput('needs_repro', 'false');
    return;
  }

  const client = new Anthropic({ apiKey: ANTHROPIC_API_KEY });

  const response = await client.messages.create({
    model: 'claude-3-sonnet-20240229',
    max_tokens: 1024,
    messages: [{
      role: 'user',
      content: `Analyze this GitHub issue and return ONLY valid JSON (no markdown):

Title: ${ISSUE_TITLE}
Body: ${ISSUE_BODY}

Return JSON with this exact format:
{"labels": ["bug", "enhancement", "question", "priority:high", "area:frontend"], "needsReproSteps": true, "summary": "brief summary"}`
    }]
  });

  let result;
  try {
    const text = response.content[0].text;
    const jsonMatch = text.match(/\{[\s\S]*\}/);
    if (jsonMatch) {
      result = JSON.parse(jsonMatch[0]);
    } else {
      result = JSON.parse(text);
    }
  } catch (e) {
    console.log('Failed to parse AI response, using defaults');
    result = { labels: [], needsReproSteps: false };
  }

  const { github } = require('@actions/github');

  if (result.labels && result.labels.length > 0) {
    await github.rest.issues.addLabels({
      owner: OWNER,
      repo: REPO,
      issue_number: parseInt(ISSUE_NUMBER, 10),
      labels: result.labels
    });
    console.log('Labels applied:', result.labels.join(', '));
  }

  setOutput('labels', result.labels ? result.labels.join(',') : '');
  setOutput('needs_repro', result.needsReproSteps ? 'true' : 'false');

  console.log('Triage complete');
}

function setOutput(name, value) {
  console.log(`::set-output name=${name}::${value}`);
}

analyzeIssue().catch(err => {
  console.error('Triage failed:', err.message);
  setOutput('labels', '');
  setOutput('needs_repro', 'false');
});
