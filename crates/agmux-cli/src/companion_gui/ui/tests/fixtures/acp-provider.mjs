// External provider stand-in: protocol/error tests only, never model acceptance.
import readline from 'node:readline';
import fs from 'node:fs';
const acp=process.argv.includes('--acp');
if (!acp) {
  if (process.argv.includes('--version')) {console.log('ACP test fixture (not Gemini)');process.exit(0);}
  process.stdout.write('\x1b[?2004h┌──────────┐\r\n│ Terminal │\r\n└──────────┘\r\nNative CLI fixture only\r\n');
  if (process.stdin.isTTY) process.stdin.setRawMode(true);
  let input=Buffer.alloc(0);
  process.stdin.on('data', data => {
    input=Buffer.concat([input,Buffer.from(data)]);
    let enter;
    while((enter=input.indexOf(13))>=0) {
      process.stdout.write(`INPUT:${input.subarray(0,enter+1).toString('hex')}\r\n`);
      input=input.subarray(enter+1);
    }
  });
} else {
  let turn=0, active=null, permission=null, ignoreCancel=false;
  const output=value=>process.stdout.write(JSON.stringify({jsonrpc:'2.0',...value})+'\n');
  const respond=(id,result)=>output({id,result});
  const chunk=text=>output({method:'session/update',params:{sessionId:'fixture-session',update:{sessionUpdate:'agent_message_chunk',content:{type:'text',text}}}});
  const finish=(reason='end_turn')=>{if(active!==null){respond(active,{stopReason:reason});active=null;}};
  readline.createInterface({input:process.stdin}).on('line', line=>{
    const req=JSON.parse(line);
    if(req.method==='initialize') {if(!fs.existsSync('hang-initialize')) respond(req.id,{protocolVersion:1,agentCapabilities:{},authMethods:[]});}
    else if(req.method==='session/new') {
      if(req.params.cwd.startsWith('\\\\?\\')) {output({id:req.id,error:{code:-32602,message:'Node-facing project paths must not be Windows verbatim paths'}});return;}
      const at=process.argv.indexOf('--model');
      if(fs.existsSync('no-model-inventory')) {respond(req.id,{sessionId:'fixture-session'});return;}
      respond(req.id,{sessionId:'fixture-session',models:{currentModelId:at>=0?process.argv[at+1]:'provider-default',availableModels:[{modelId:'provider-default',name:'Provider default'},{modelId:'fixture-fast',name:'Fixture fast'}]}});
    } else if(req.method==='session/prompt') {
      turn++;active=req.id;const text=req.params.prompt[0].text;
      fs.appendFileSync('acp-requests.jsonl',JSON.stringify({turn,text,args:process.argv})+'\n');
      if(text==='reject-provider') {output({id:req.id,error:{code:-32000,message:'Provider rejected this request'}});active=null;}
      else if(text==='exit-provider') process.exit(23);
      else if(text==='wait-for-cancel' || text==='ignore-cancel') {ignoreCancel=text==='ignore-cancel';}
      else if(text==='ask-permission') {permission=600;output({id:permission,method:'session/request_permission',params:{sessionId:'fixture-session',toolCall:{toolCallId:'tool-1',title:'Write project file'},options:[{optionId:'allow-once',name:'Allow once',kind:'allow_once'},{optionId:'reject-once',name:'Reject',kind:'reject_once'}]}});}
      else {setTimeout(()=>{chunk(`Response ${turn}: `);setTimeout(()=>{chunk(text);finish();},50);},80);}
    } else if(req.method==='session/cancel') {if(!ignoreCancel) finish('cancelled');}
    else if(req.id===permission && req.result) {chunk(`Permission: ${req.result.outcome.optionId}`);permission=null;finish();}
    else if(req.id!==undefined) output({id:req.id,error:{code:-32601,message:'Unsupported fixture method'}});
  });
}
