// F2 — capture RESULT popup. Same multi-engine panel as F1, with an// OCR preview strip above showing what was captured.
const PopupF2Artboard = ({ dark }) => {
  const fg = dark ? '#f0ebdf' : '#1c1a16';
  const sub = dark ? 'rgba(240,235,223,.6)' : 'rgba(30,24,14,.58)';
  const muted = dark ? 'rgba(240,235,223,.4)' : 'rgba(30,24,14,.4)';
  const border = dark ? 'rgba(255,255,255,.08)' : 'rgba(30,24,14,.08)';
  const bg = dark ? 'rgba(26,24,20,.94)' : 'rgba(250,248,244,.96)';

  // For capture: multiline editable source (already OCR-filled). Roster matches
  // Settings / F1 so the same app shows the same engines.
  const engines = [
    { name:'LM Studio（本地）', badge:'LM', grad:'linear-gradient(135deg, #4ea3d9, #2d7fb3)', meta:'qwen2.5:7b · 842ms', expanded: true, state:'ok', primary:'组件会经历挂载、更新和卸载。Hook 按可预测的顺序触发，副作用应在下次渲染前被清理。', isDefault: true },
    { name:'DeepSeek', badge:'DS', grad:'linear-gradient(135deg, #8a7fe8, #6356c8)', meta:'deepseek-chat · 1.1s', expanded: true, state:'ok', primary:'组件会挂载、更新和卸载。Hook 以可预测的顺序触发，副作用应在下次渲染周期开始前清理。' },
    { name:'Google 翻译', badge:'G', grad:'linear-gradient(135deg, #4285f4, #1a73e8)', meta:'en → zh · 201ms', expanded: true, state:'ok', primary:'组件挂载、更新和卸载。钩子按可预测的顺序触发，副作用应在下一个渲染周期开始之前清除。' },
    { name:'Ollama', badge:'OL', grad:'linear-gradient(135deg, #45c08a, #2f9d6e)', meta:'llama3.1:8b · 180ms', expanded: false, state:'ok', primary:'组件挂载、更新与卸载。钩子按可预测顺序触发，副作用应在下一渲染周期前清理。' },
  ];

  return (
    <div style={{
      width: 460,
      background: bg,
      backdropFilter:'saturate(1.4) blur(30px)',
      border:`0.5px solid ${border}`,
      borderRadius: 12, overflow:'hidden',
      boxShadow:'0 24px 48px -12px rgba(30,24,14,.25), 0 2px 6px rgba(30,24,14,.1)',
      color: fg, fontFamily:'var(--sf)',
    }}>
      {/* Header */}
      <div style={{ padding:'10px 14px', display:'flex', alignItems:'center', gap: 10, borderBottom:`0.5px solid ${border}` }}>
        <LensMark size={16}/>
        <span style={{ fontSize: 12, fontWeight: 600, color: fg, letterSpacing:'-.01em' }}>Lenz</span>
        <div style={{ marginLeft:'auto', display:'flex', alignItems:'center', gap: 8 }}>
          <button style={{
            padding:'4px 9px', fontSize: 11, border:'none', borderRadius: 5,
            background:'var(--lens)', color:'#fff', cursor:'pointer',
            display:'flex', gap: 5, alignItems:'center', fontFamily:'var(--sf)', fontWeight: 500,
            boxShadow:'0 1px 2px rgba(45,127,179,.3), inset 0 1px 0 rgba(255,255,255,.2)',
          }}><Icon name="overlay" size={12}/>覆盖到原图</button>
          <Icon name="close" size={13} style={{ color: muted, cursor:'pointer' }}/>
        </div>
      </div>

      {/* OCR preview — the captured image */}
      <div style={{ padding:'10px 14px 0' }}>
        <div style={{
          position:'relative', borderRadius: 7,
          border:`0.5px solid ${border}`, overflow:'hidden', background:'#fff',
        }}>
          <div style={{ padding:'10px 12px', color:'#1c1a16' }}>
            <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 4 }}>Component lifecycle</div>
            <div style={{ fontSize: 9.5, color:'#6b6252', lineHeight: 1.5 }}>
              Components mount, update, and unmount. Hooks fire in a predictable order, and side effects
              should be cleaned up before the next render cycle begins.
            </div>
          </div>
          <div style={{ position:'absolute', top: 8, left: 10, right: 10, height: 13, border:'1px dashed rgba(78,163,217,.4)', borderRadius: 2 }}/>
          <div style={{ position:'absolute', top: 26, left: 10, right: 10, height: 32, border:'1px dashed rgba(78,163,217,.35)', borderRadius: 2 }}/>
          <div style={{
            position:'absolute', top: 6, right: 8,
            padding:'1px 5px', fontSize: 9, fontFamily:'var(--mono)',
            background:'rgba(78,163,217,.9)', color:'#fff', borderRadius: 3,
          }}>2 区块 · 99%</div>
        </div>
      </div>

      {/* Source + language switch — shared block, identical to F1 */}
      <SourceLangBlock
        dark={dark}
        source="Components mount, update, and unmount. Hooks fire in a predictable order, and side effects should be cleaned up before the next render cycle begins."
        detected="英语"
        target="中文"
        note="可修正 OCR 误识"
      />

      {/* Engine modules — shared list, identical interaction to F1 */}
      <EngineList dark={dark} engines={engines} storeKey="lenz.f2.openMap.v1" />

      {/* Footer */}
      <div style={{
        padding:'7px 14px', display:'flex', alignItems:'center', gap: 8,
        borderTop:`0.5px solid ${border}`,
        background: dark ? 'rgba(0,0,0,.2)' : 'rgba(30,24,14,.02)',
        fontSize: 10, color: muted,
      }}>
        <Icon name="overlay" size={11}/>
        <span style={{ cursor:'pointer' }}>覆盖到原图</span>
        <span style={{ marginLeft:'auto', display:'flex', gap: 4, alignItems:'center' }}>
          <Kbd keys={['esc']} dark={dark}/>
        </span>
      </div>
    </div>
  );
};

window.PopupF2Artboard = PopupF2Artboard;
