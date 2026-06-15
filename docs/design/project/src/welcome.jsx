// Welcome / onboarding — first-run
const WelcomeArtboard = ({ dark }) => {
  const bg = dark ? '#1a1814' : '#faf8f4';
  const fg = dark ? '#f0ebdf' : '#1c1a16';
  const sub = dark ? 'rgba(240,235,223,.62)' : 'rgba(30,24,14,.6)';
  const border = dark ? 'rgba(255,255,255,.08)' : 'rgba(30,24,14,.08)';
  const cardBg = dark ? 'rgba(255,255,255,.04)' : 'rgba(30,24,14,.025)';

  const steps = [
    { n: 1, t: '选择翻译引擎', d: '本地接入 LM Studio / Ollama，或填入任意 OpenAI 兼容的 Key。' },
    { n: 2, t: '设置快捷键', d: '划词、截图、原图覆盖三个功能，默认已配好。' },
    { n: 3, t: '授予 OCR 权限', d: 'macOS 会请求“屏幕录制”权限，以供 Vision 读取像素。' },
  ];

  return (
    <div className={`ln-win ${dark ? 'ln-vibrancy-dark' : 'ln-vibrancy-light'}`} style={{ width: 720, height: 520, color: fg, position:'relative' }}>
      <WinChrome title="欢迎使用 Lenz" dark={dark} />

      <div style={{ padding: '44px 56px 0', display:'flex', gap: 44, alignItems:'flex-start' }}>
        <div style={{ flex:'0 0 auto' }}>
          <img
            src="src/assets/logo.png"
            alt="Lenz"
            width={108}
            height={108}
            draggable={false}
            style={{ display:'block', width: 108, height: 108, objectFit:'contain', userSelect:'none', WebkitUserDrag:'none' }}
          />
        </div>

        <div style={{ flex: 1 }}>
          <div style={{ fontSize: 11, letterSpacing:'.14em', textTransform:'uppercase', color: sub, marginBottom: 10, fontFamily:'var(--mono)' }}>
            v0.1 · 开源免费
          </div>
          <h1 style={{ fontSize: 38, margin: '0 0 10px', fontWeight: 600, letterSpacing:'-.02em', lineHeight: 1 }}>
            Lenz
          </h1>
          <p style={{ fontSize: 15, color: fg, margin: '0 0 6px', lineHeight: 1.5, fontWeight: 500 }}>
            像镜头一样，一眼看懂世界。
          </p>
          <p style={{ fontSize: 13, color: sub, margin: 0, lineHeight: 1.5, fontFamily:'var(--sf)' }}>
            跨平台、本地优先、开源免费的截图翻译工具。
          </p>
        </div>
      </div>

      <div style={{ padding: '32px 56px 24px' }}>
        <div style={{ display:'grid', gridTemplateColumns:'repeat(3, 1fr)', gap: 10 }}>
          {steps.map(s => (
            <div key={s.n} style={{
              padding: '14px 14px 16px', borderRadius: 10,
              background: cardBg,
              border: `0.5px solid ${border}`,
            }}>
              <div style={{
                width: 22, height: 22, borderRadius:'50%',
                background: dark ? 'rgba(78,163,217,.2)' : 'rgba(78,163,217,.18)',
                color: 'var(--lens-dark)',
                display:'flex', alignItems:'center', justifyContent:'center',
                fontSize: 11, fontWeight: 600, fontFamily:'var(--mono)',
                marginBottom: 10,
              }}>{s.n}</div>
              <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 4 }}>{s.t}</div>
              <div style={{ fontSize: 11.5, color: sub, lineHeight: 1.45 }}>{s.d}</div>
            </div>
          ))}
        </div>
      </div>

      <div style={{
        position:'absolute', bottom: 0, left: 0, right: 0,
        padding: '16px 24px',
        borderTop: `0.5px solid ${border}`,
        background: dark ? 'rgba(20,18,14,.5)' : 'rgba(250,248,244,.5)',
        display:'flex', alignItems:'center', justifyContent:'space-between',
      }}>
        <div style={{ fontSize: 12, color: sub, display:'flex', alignItems:'center', gap: 6 }}>
          <Icon name="check" size={14} style={{ color:'var(--sage)' }} />
          完全运行在本机，你的屏幕永远不会离开设备。
        </div>
        <div style={{ display:'flex', gap: 8 }}>
          <button style={{
            padding:'7px 14px', fontSize: 12.5, border:'none', borderRadius: 7,
            background: 'transparent', color: fg, cursor:'pointer',
            fontFamily:'var(--sf)', fontWeight: 500,
          }}>跳过</button>
          <button style={{
            padding:'7px 16px', fontSize: 12.5, border:'none', borderRadius: 7,
            background:'var(--lens)', color:'#fff', cursor:'pointer',
            fontFamily:'var(--sf)', fontWeight: 500,
            boxShadow:'0 1px 2px rgba(45,127,179,.3), inset 0 1px 0 rgba(255,255,255,.2)',
          }}>开始使用</button>
        </div>
      </div>
    </div>
  );
};

window.WelcomeArtboard = WelcomeArtboard;
