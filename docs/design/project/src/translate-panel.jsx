// Shared translation UI — used by F1 (word select) and F2 (capture result).
//
// EngineList is the reusable lower section: a list of collapsible engine
// modules. Rule: expanded = auto-translated (shows result + actions);
// collapsed = not translated. Collapse state is remembered per engine in
// localStorage (keyed by `storeKey`) so the panel reopens as the user left it.

const EngineList = ({ dark, engines, storeKey = 'lenz.engines.openMap.v1' }) => {
  const fg = dark ? '#f0ebdf' : '#1c1a16';
  const sub = dark ? 'rgba(240,235,223,.62)' : 'rgba(30,24,14,.58)';
  const muted = dark ? 'rgba(240,235,223,.42)' : 'rgba(30,24,14,.42)';
  const border = dark ? 'rgba(255,255,255,.08)' : 'rgba(30,24,14,.08)';
  const rowSoft = dark ? 'rgba(255,255,255,.025)' : 'rgba(30,24,14,.02)';

  const [openMap, setOpenMap] = React.useState(() => {
    let saved = {};
    try { saved = JSON.parse(localStorage.getItem(storeKey) || '{}'); } catch (e) {}
    const init = {};
    engines.forEach(e => { init[e.name] = (e.name in saved) ? saved[e.name] : !!e.expanded; });
    return init;
  });
  React.useEffect(() => {
    try { localStorage.setItem(storeKey, JSON.stringify(openMap)); } catch (e) {}
  }, [openMap, storeKey]);
  const toggle = (name) => setOpenMap(m => ({ ...m, [name]: !m[name] }));

  const ActionIcon = ({ name, title }) => (
    <span title={title} style={{ display:'inline-flex', cursor:'pointer' }}>
      <Icon name={name} size={13} style={{ color: muted }} />
    </span>
  );

  const EngineRow = ({ e, isLast }) => {
    const open = openMap[e.name];
    return (
      <div className="ln-engrow" style={{
        borderBottom: isLast ? 'none' : `0.5px solid ${border}`,
        background: open ? 'transparent' : rowSoft,
        borderLeft: e.isDefault ? '2px solid var(--lens)' : '2px solid transparent',
      }}>
        <div onClick={() => toggle(e.name)}
          style={{ display:'flex', alignItems:'center', gap: 8, padding:'9px 12px', cursor:'pointer' }}>
          <span style={{
            width: 18, height: 18, borderRadius: 4,
            background: e.grad, color:'#fff',
            fontFamily:'var(--mono)', fontSize: 9, fontWeight: 700,
            display:'flex', alignItems:'center', justifyContent:'center',
          }}>{e.badge}</span>
          <span style={{ fontSize: 12, fontWeight: 600, color: fg }}>{e.name}</span>
          {e.isDefault && (
            <span style={{
              fontSize: 8.5, fontFamily:'var(--mono)', letterSpacing:'.06em',
              padding:'1px 5px', borderRadius: 3,
              background: dark ? 'rgba(78,163,217,.22)' : 'rgba(78,163,217,.15)',
              color:'var(--lens-dark)', fontWeight: 700,
            }}>默认</span>
          )}
          <span style={{ marginLeft:'auto', fontSize: 10, color: muted, fontFamily:'var(--mono)', display:'flex', alignItems:'center', gap: 5 }}>
            {open && e.state === 'streaming' && (
              <span style={{ width: 5, height: 5, borderRadius:'50%', background:'var(--lens)', boxShadow:'0 0 4px var(--lens)' }}/>
            )}
            {open && e.state === 'ok' && <Icon name="check" size={10} style={{ color:'var(--sage)' }}/>}
            {open && <span>{e.meta}</span>}
            <Icon name={open ? 'chevronDown' : 'chevronRight'} size={12} style={{ color: muted, marginLeft: 2 }}/>
          </span>
        </div>

        {open && (
          <div style={{ padding:'0 12px 11px 38px' }}>
            <div style={{ fontSize: 13.5, color: fg, lineHeight: 1.5, display:'flex', alignItems:'baseline', gap: 6 }}>
              <span>{e.primary}</span>
              {e.state === 'streaming' && (
                <span style={{ display:'inline-block', width: 7, height: 13, background:'var(--lens)', borderRadius: 1, opacity: .9 }}/>
              )}
            </div>
            <div style={{ display:'flex', alignItems:'center', gap: 13, marginTop: 9 }}>
              <ActionIcon name="volume" title="朗读译文" />
              <ActionIcon name="copy" title="复制译文" />
              <ActionIcon name="insert" title="使用译文替换原文" />
            </div>
          </div>
        )}
      </div>
    );
  };

  return (
    <div>
      {engines.map((e, i) => <EngineRow key={e.name} e={e} isLast={i === engines.length - 1}/>)}
    </div>
  );
};

// Shared upper section: editable source field + action row (read / copy /
// detected-language chip / char count) + language switch bar. Used by both
// F1 (word select) and F2 (capture result) so the two stay identical.
const SourceLangBlock = ({ dark, source = '', detected = '英语', target = '中文', note }) => {
  const fg = dark ? '#f0ebdf' : '#1c1a16';
  const sub = dark ? 'rgba(240,235,223,.62)' : 'rgba(30,24,14,.58)';
  const muted = dark ? 'rgba(240,235,223,.42)' : 'rgba(30,24,14,.42)';
  const border = dark ? 'rgba(255,255,255,.08)' : 'rgba(30,24,14,.08)';
  const fieldBg = dark ? 'rgba(0,0,0,.22)' : 'rgba(30,24,14,.025)';

  const LangPick = ({ label, accent }) => (
    <div style={{
      display:'inline-flex', alignItems:'center', gap: 5, cursor:'pointer',
      fontSize: 12.5, fontWeight: 600, color: accent ? 'var(--lens-dark)' : fg,
    }}>
      {label}
      <Icon name="chevronDown" size={11} style={{ color: muted }} />
    </div>
  );

  return (
    <>
      {/* Editable source — rich-text field */}
      <div style={{ padding:'11px 12px', borderBottom:`0.5px solid ${border}`, background: fieldBg }}>
        <div
          contentEditable suppressContentEditableWarning
          className="ln-source-field"
          style={{
            fontSize: 13, color: fg, lineHeight: 1.55, outline:'none',
            minHeight: 38, whiteSpace:'pre-wrap', wordBreak:'break-word',
            caretColor:'var(--lens)',
            border:`1px solid ${dark ? 'rgba(255,255,255,.14)' : 'rgba(30,24,14,.16)'}`,
            borderRadius: 7, padding:'8px 10px',
            background: dark ? 'rgba(0,0,0,.2)' : '#fff',
          }}
        >{source}</div>
        <div style={{ marginTop: 9, display:'flex', alignItems:'center', gap: 11 }}>
          <Icon name="volume" size={13} style={{ color: muted, cursor:'pointer' }}/>
          <Icon name="copy" size={13} style={{ color: muted, cursor:'pointer' }}/>
          <span style={{
            display:'inline-flex', alignItems:'center', gap: 5,
            padding:'2px 8px', borderRadius: 5,
            background: dark ? 'rgba(255,255,255,.06)' : 'rgba(30,24,14,.05)',
            fontSize: 11, color: sub,
          }}>
            识别为 <span style={{ color:'var(--lens-dark)', fontWeight: 600 }}>{detected}</span>
          </span>
          {note && (
            <span style={{ fontSize: 10.5, color: muted, fontStyle:'italic' }}>{note}</span>
          )}
          <span style={{ marginLeft:'auto', fontSize: 10, color: muted, fontFamily:'var(--mono)' }}>{(source || '').length} 字符</span>
        </div>
      </div>

      {/* Language switch bar */}
      <div style={{
        display:'flex', alignItems:'center', padding:'9px 18px',
        borderBottom:`0.5px solid ${border}`,
        background: dark ? 'rgba(0,0,0,.12)' : 'rgba(30,24,14,.015)',
      }}>
        <div style={{ flex: 1, display:'flex', justifyContent:'center' }}><LangPick label="自动检测"/></div>
        <Icon name="swap" size={15} style={{ color: sub, cursor:'pointer' }}/>
        <div style={{ flex: 1, display:'flex', justifyContent:'center' }}><LangPick label={target} accent/></div>
      </div>
    </>
  );
};

const TranslatePanel = ({
  dark,
  width = 380,
  source,
  detected = '英语',
  target = '中文',
  engines,
  fromInput = false,
  showTail = false,
  footer = true,
}) => {
  const fg = dark ? '#f0ebdf' : '#1c1a16';
  const sub = dark ? 'rgba(240,235,223,.62)' : 'rgba(30,24,14,.58)';
  const muted = dark ? 'rgba(240,235,223,.42)' : 'rgba(30,24,14,.42)';
  const border = dark ? 'rgba(255,255,255,.08)' : 'rgba(30,24,14,.08)';
  const bg = dark ? 'rgba(26,24,20,.94)' : 'rgba(250,248,244,.96)';

  // Pin toggle: unpinned = popup follows the cursor; pinned = stays at the
  // last fixed position.
  const [pinned, setPinned] = React.useState(false);

  return (
    <div style={{
      width,
      background: bg,
      backdropFilter:'saturate(1.4) blur(30px)',
      border:`0.5px solid ${border}`,
      borderRadius: 11,
      boxShadow:'0 18px 44px -10px rgba(30,24,14,.3), 0 2px 6px rgba(30,24,14,.12)',
      overflow:'hidden', position:'relative',
      color: fg, fontFamily:'var(--sf)',
    }}>
      {showTail && (
        <div style={{
          position:'absolute', top: -6, left: 40, width: 12, height: 12,
          background: bg, transform:'rotate(45deg)',
          borderLeft:`0.5px solid ${border}`, borderTop:`0.5px solid ${border}`,
        }}/>
      )}

      {/* Header — brand + window actions */}
      <div style={{ padding:'9px 12px', borderBottom:`0.5px solid ${border}`, display:'flex', alignItems:'center', gap: 8 }}>
        <LensMark size={15}/>
        <span style={{ fontSize: 12, fontWeight: 600, color: fg, letterSpacing:'-.01em' }}>Lenz</span>
        {pinned && (
          <span style={{
            display:'inline-flex', alignItems:'center', gap: 4,
            fontSize: 10, color:'var(--lens-dark)', fontWeight: 500,
            padding:'1px 6px', borderRadius: 4,
            background: dark ? 'rgba(78,163,217,.18)' : 'rgba(78,163,217,.12)',
          }}>已钉住</span>
        )}
        <div style={{ marginLeft:'auto', display:'flex', alignItems:'center', gap: 11, color: muted }}>
          <button
            onClick={() => setPinned(p => !p)}
            title={pinned ? '已钉住 · 固定在上次位置' : '钉住窗口 · 当前跟随鼠标'}
            style={{
              display:'inline-flex', alignItems:'center', justifyContent:'center',
              width: 22, height: 22, padding: 0, borderRadius: 5, cursor:'pointer',
              border:'none',
              background: pinned ? 'var(--lens)' : 'transparent',
              color: pinned ? '#fff' : muted,
            }}>
            <Icon name="pin" size={13}/>
          </button>
          <Icon name="star" size={13} style={{ cursor:'pointer' }}/>
          <Icon name="close" size={13} style={{ cursor:'pointer' }}/>
        </div>
      </div>

      {/* Source + language switch — shared block */}
      <SourceLangBlock dark={dark} source={source} detected={detected} target={target} />

      {/* Engine modules — shared list */}
      <EngineList dark={dark} engines={engines} storeKey="lenz.f1.openMap.v1" />

      {footer && (
        <div style={{
          padding:'6px 12px', display:'flex', alignItems:'center', gap: 8,
          borderTop:`0.5px solid ${border}`,
          background: dark ? 'rgba(0,0,0,.2)' : 'rgba(30,24,14,.02)',
          fontSize: 10, color: muted,
        }}>
          <Icon name="info" size={10}/>
          <span>折叠的引擎不会自动翻译 · 记住折叠状态</span>
          <span style={{ marginLeft:'auto', display:'flex', gap: 4, alignItems:'center' }}>
            <Kbd keys={['esc']} dark={dark}/>
          </span>
        </div>
      )}
    </div>
  );
};

// Shared demo engines — roster matches the services configured in Settings
// (LM Studio + Ollama local, DeepSeek + Google cloud). Keep this in sync with
// settings.jsx EnginesPane and popup-f2.jsx so the same app shows the same engines.
// Rule: expanded = auto-translated (shows result); collapsed = not translated.
const DEMO_ENGINES = [
  { name:'LM Studio（本地）', badge:'LM', grad:'linear-gradient(135deg, #4ea3d9, #2d7fb3)', meta:'qwen2.5:7b · 512ms', expanded: true, state:'ok', primary:'有主见的默认设置', isDefault: true },
  { name:'Ollama', badge:'OL', grad:'linear-gradient(135deg, #45c08a, #2f9d6e)', meta:'llama3.1:8b · streaming', expanded: true, state:'streaming', primary:'有主见的缺省值' },
  { name:'DeepSeek', badge:'DS', grad:'linear-gradient(135deg, #8a7fe8, #6356c8)', meta:'deepseek-chat · 410ms', expanded: false, state:'ok', primary:'自以为是的默认值' },
  { name:'Google 翻译', badge:'G', grad:'linear-gradient(135deg, #4285f4, #1a73e8)', meta:'en → zh · 90ms', expanded: false, state:'ok', primary:'固执己见的默认设置' },
];

window.EngineList = EngineList;
window.SourceLangBlock = SourceLangBlock;
window.TranslatePanel = TranslatePanel;
window.DEMO_ENGINES = DEMO_ENGINES;
