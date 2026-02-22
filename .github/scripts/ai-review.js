const { Anthropic } = require('@anthropic-ai/sdk');

async function runAIReview() {
  const ANTHROPIC_API_KEY = process.env.ANTHROPIC_API_KEY;
  const FILES = process.env.FILES || '';
  const DIFF = process.env.DIFF || '';
  const PR_NUMBER = process.env.PR_NUMBER;
  const OWNER = process.env.OWNER;
  const REPO = process.env.REPO;

  if (!ANTHROPIC_API_KEY) {
    console.log('ANTHROPIC_API_KEY not set, skipping AI review');
    return;
  }

  if (!DIFF || DIFF.trim() === '') {
    console.log('No code changes to review');
    return;
  }

  const client = new Anthropic({ apiKey: ANTHROPIC_API_KEY });

  const response = await client.messages.create({
    model: 'claude-3-sonnet-20240229',
    max_tokens: 4096,
    messages: [{
      role: 'user',
      content: `You are a senior Rust developer reviewing a pull request. Provide a thorough code review.

Changed files:
${FILES}

Diff:
${DIFF}

Provide a review with these sections:
1. **Summary**: Brief overview of changes
2. **What looks good**: Positive aspects of the code
3. **Potential issues**: Bugs, edge cases, performance concerns
4. **Suggestions**: Improvements and refactoring ideas
5. **Security concerns**: Any security issues

Format as GitHub markdown. Be concise but helpful.`
    }]
  });

  const { github } = require('@actions/github');
  
  await github.rest.pulls.createReview({
    owner: OWNER,
    repo: REPO,
    pull_number: parseInt(PR_NUMBER, 10),
    body: response.content[0].text,
    event: 'COMMENT'
  });

  console.log('AI review posted successfully');
}

runAIReview().catch(err => {
  console.error('AI review failed:', err.message);
  process.exit(1);
});
