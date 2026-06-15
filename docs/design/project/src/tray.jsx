// Tray / menu-bar dropdown
const TrayArtboard = ({ dark }) => {
  const fg = dark ? '#f0ebdf' : '#1c1a16';
  const sub = dark ? 'rgba(240,235,223,.6)' : 'rgba(30,24,14,.56)';
  const border = dark ? 'rgba(255,255,255,.08)' : 'rgba(30,24,14,.08)';
  const bg = dark ? 'rgba(30,28,24,.92)' : 'rgba(250,248,244,.94)';

  const Item = ({ icon, label, kbd, divider, muted, danger }) => (
    <div style={{
      display:'flex', alignItems:'center', gap: 10,
      padding:'6px 10px 6px 12px', fontSize: 13,
      color: danger ? '#c04040' : (muted ? sub : fg),
      borderRadius: 5, cursor:'pointer',
    }}>
      {icon && <Icon name={icon} size={14} style={{ color: danger ? '#c04040' : sub }}/>}
      <span style={{ flex: 1 }}>{label}</span>
      {kbd && <Kbd keys={kbd} dark={dark}/>}
    </div>
  );

  const Divider = () => <div style={{ height: 1, background: border, margin:'5px 8px' }}/>;

  return (
    <div style={{
      width: 300, position:'relative',
      fontFamily:'var(--sf)',
    }}>
      {/* Fake menu-bar with tray icon */}
      <div style={{
        position:'absolute', top: -32, right: 8,
        padding:'6px 10px', borderRadius: 5,
        background: dark ? 'rgba(255,255,255,.15)' : 'rgba(30,24,14,.1)',
        display:'flex', alignItems:'center', gap: 5,
      }}>
        <LensMark size={15}/>
      </div>

      <div style={{
        background: bg,
        backdropFilter:'saturate(1.4) blur(30px)',
        border:`0.5px solid ${border}`,
        borderRadius: 10,
        padding: 5,
        boxShadow:'0 24px 48px -12px rgba(30,24,14,.28), 0 2px 6px rgba(30,24,14,.1)',
        color: fg,
      }}>
        {/* Status header */}
        <div style={{
          padding:'10px 10px 12px',
          display:'flex', alignItems:'center', gap: 10,
          borderBottom:`0.5px solid ${border}`, marginBottom: 4,
        }}>
          <LensMark size={24}/>
          <div style={{ flex: 1 }}>
            <div style={{ fontSize: 13, fontWeight: 600 }}>Lenz</div>
            <div style={{ fontSize: 11, color: sub, display:'flex', alignItems:'center', gap: 5 }}>
              <span style={{ width:6, height:6, borderRadius:'50%', background:'var(--sage)', boxShadow:'0 0 4px var(--sage)'}}/>
              LM Studio · 空闲
            </div>
          </div>
        </div>

        <Item icon="selection" label="划词翻译"   kbd={['⌥','D']} />
        <Item icon="capture"   label="截图翻译"   kbd={['⌥','S']} />
        <Item icon="overlay"   label="原图覆盖翻译" kbd={['⌥','F']} />

        <Divider/>

        {/* engine picker */}
        <div style={{ padding:'4px 10px 6px' }}>
          <div style={{ fontSize: 10, color: sub, letterSpacing:'.06em', fontFamily:'var(--mono)', marginBottom: 4 }}>翻译引擎</div>
          <div style={{
            display:'flex', alignItems:'center', gap: 8,
            padding:'5px 8px', borderRadius: 5,
            background: dark ? 'rgba(78,163,217,.14)' : 'rgba(78,163,217,.1)',
            fontSize: 12,
          }}>
            <div style={{
              width: 16, height: 16, borderRadius: 4,
              background:'linear-gradient(135deg, #4ea3d9, #2d7fb3)',
              color:'#fff', display:'flex', alignItems:'center', justifyContent:'center',
              fontFamily:'var(--mono)', fontSize: 8, fontWeight: 700,
            }}>LM</div>
            <span style={{ flex: 1, fontWeight: 500 }}>LM Studio</span>
            <span style={{ fontSize: 10.5, fontFamily:'var(--mono)', color: sub }}>qwen2.5:7b</span>
            <Icon name="chevronDown" size={11} style={{ color: sub }}/>
          </div>
        </div>

        <Divider/>

        <Item icon="settings" label="设置…" kbd={['⌥','⌘','T']}/>
        <Item icon="info" label="关于 Lenz" />
        <Item icon="external" label="退出 Lenz" kbd={['⌘','Q']} danger />
      </div>
    </div>
  );
};

window.TrayArtboard = TrayArtboard;
