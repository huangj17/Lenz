// F1 — word selection popup (uses shared TranslatePanel)
const PopupF1Artboard = ({ dark }) => {
  const fg = dark ? '#f0ebdf' : '#1c1a16';
  const sub = dark ? 'rgba(240,235,223,.62)' : 'rgba(30,24,14,.58)';
  const border = dark ? 'rgba(255,255,255,.08)' : 'rgba(30,24,14,.08)';

  return (
    <div style={{
      width: 560, height: 600, position:'relative',
      background: dark ? '#1a1814' : '#faf8f4',
      borderRadius: 12, overflow:'hidden',
      border:`0.5px solid ${border}`,
      boxShadow:'0 24px 48px -12px rgba(30,24,14,.18), 0 2px 6px rgba(30,24,14,.08)',
      color: fg, fontFamily:'var(--sf)',
    }}>
      <WinChrome title="撰写回复" dark={dark}/>
      <div style={{ padding:'18px 24px' }}>
        <div style={{ fontSize: 11, color: sub, marginBottom: 8, fontFamily:'var(--mono)', letterSpacing:'.04em' }}>收件人：team@acme.dev</div>
        <div style={{
          border:`1px solid ${dark ? 'rgba(255,255,255,.12)' : 'rgba(30,24,14,.14)'}`,
          borderRadius: 8, padding:'12px 14px', minHeight: 90,
          background: dark ? 'rgba(0,0,0,.18)' : '#fff',
          fontSize: 13, lineHeight: 1.7, color: fg,
        }}>
          Thanks for the detailed write-up. We committed to <span style={{
            background: dark ? 'rgba(78,163,217,.28)' : 'rgba(78,163,217,.22)',
            boxShadow:'inset 0 -2px 0 var(--lens)', padding:'0 1px',
          }}>opinionated defaults</span> here, so most of those knobs won't ship.
          <span style={{
            display:'inline-block', width: 1.5, height: 15, background:'var(--lens)',
            marginLeft: 1, verticalAlign:'-3px',
          }}/>
        </div>
        <div style={{ marginTop: 8, fontSize: 11, color: sub, display:'flex', alignItems:'center', gap: 6 }}>
          <Icon name="info" size={12}/>选中输入框里的文本 → 划词翻译
        </div>
      </div>

      <div style={{ position:'absolute', top: 108, left: 158 }}>
        <TranslatePanel
          dark={dark}
          width={380}
          source="opinionated defaults"
          engines={DEMO_ENGINES}
          fromInput
          showTail
        />
      </div>
    </div>
  );
};

window.PopupF1Artboard = PopupF1Artboard;
