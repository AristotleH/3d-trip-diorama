// Explicit opt-in: this runner makes paid/provider calls, never runs in CI.
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { createLLM } from '../server/llm.mjs';
const cases = JSON.parse(await readFile(new URL('../tests/evals/prompts.json', import.meta.url)));
const args = process.argv.slice(2);
if (!args.includes('--live')) {
  console.log('Dry run: no requests sent. Use --live to call the configured provider. Optional --case=<id>.');
  console.log(cases.map(c => `${c.id}: ${c.prompt}`).join('\n'));
  process.exit(0);
}
if (!process.env.LLM_MODEL) throw new Error('Set LLM_MODEL and provider credentials first.');
const selected = args.find(a => a.startsWith('--case='))?.slice(7);
const suite = cases.filter(c => !selected || c.id === selected);
if (!suite.length) throw new Error('Unknown case ID');
const interpret = createLLM({baseUrl:process.env.LLM_BASE_URL,model:process.env.LLM_MODEL,apiKey:process.env.LLM_API_KEY,outputMode:process.env.LLM_OUTPUT_MODE});
const directory = new URL(`../test-results/llm-${Date.now()}/`, import.meta.url);
await mkdir(directory,{recursive:true});
const report=[];
for (const item of suite) {
  const start=performance.now();
  try {
    const output=await interpret(item.prompt,item.mode,AbortSignal.timeout(90000));
    const failures=[];
    if(item.radius && output.radius_m!==item.radius) failures.push(`Expected radius ${item.radius}`);
    if(item.clarify && output.query!=='NEEDS_CLARIFICATION') failures.push('Expected clarification');
    for(const term of item.queryIncludes||[]) if(!output.query.toLowerCase().includes(term)) failures.push(`Missing ${term}`);
    await writeFile(new URL(`${item.id}.json`,directory),JSON.stringify(output,null,2));
    report.push({id:item.id,ms:Math.round(performance.now()-start),status:failures.length?'failed':'passed automatic checks',failures,manualReview:item.review||null});
  } catch(error) { report.push({id:item.id,ms:Math.round(performance.now()-start),status:'failed',error:error.message}); }
}
await writeFile(new URL('report.json',directory),JSON.stringify({model:process.env.LLM_MODEL,results:report},null,2));
console.log(JSON.stringify(report,null,2));
console.log(`Saved report and outputs to ${directory.pathname}`);
if(report.some(r=>r.status==='failed')) process.exitCode=1;
