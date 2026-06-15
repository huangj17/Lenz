// F3 — In-place overlay (THE core differentiator)
// Simulated English webpage (GitHub-like) with Chinese translations
// overlaid on each text block.
const OverlayF3Artboard = ({ dark }) => {
  const W = 860, H = 560;

  // Each block: original on page, translation overlay with same bbox
  const blocks = [
    { x: 60, y: 100, w: 420, h: 28, en: 'Breaking changes in the 2.0 release', zh: '2.0 版本中的破坏性变更', size: 20, weight: 700 },
    { x: 60, y: 140, w: 620, h: 20, en: 'Pushed 2 hours ago · 14 files changed · +284 −192', zh: '2 小时前推送 · 改动 14 个文件 · +284 −192', size: 11, weight: 400, muted: true },
    { x: 60, y: 200, w: 720, h: 22, en: 'The scroll position was not properly restored when navigating between routes.', zh: '在不同路由间导航时,滚动位置未被正确恢复。', size: 14, weight: 500 },
    { x: 60, y: 240, w: 720, h: 60, en: 'We now snapshot the viewport offset in sessionStorage before each transition and replay it on the next commit. Existing usages of ScrollRestoration should continue to work without changes, but custom scroll containers will need to opt in via the new restore prop.', zh: '我们会在每次过渡之前将视口偏移保存到 sessionStorage,并在下次提交时进行回放。已有的 ScrollRestoration 用法可继续工作,但自定义滚动容器需要通过新的 restore 属性显式启用。', size: 13, weight: 400 },
    { x: 60, y: 330, w: 360, h: 20, en: 'Known issues', zh: '已知问题', size: 15, weight: 600 },
    { x: 60, y: 362, w: 700, h: 40, en: 'Safari 17 fires scroll events asynchronously after popstate, which can cause a one-frame flash in slow-loading panes.', zh: 'Safari 17 会在 popstate 之后异步触发 scroll 事件,这在加载较慢的面板中会导致一帧闪烁。', size: 12, weight: 400 },
  ];

  return (
    <div style={{
      width: W, height: H, position:'relative', borderRadius: 12, overflow:'hidden',
      boxShadow:'0 24px 48px -12px rgba(30,24,14,.22), 0 2px 6px rgba(30,24,14,.08)',
      fontFamily:'var(--sf)', background:'#f6f8fa',
    }}>
      {/* Fake GitHub-style page underneath */}
      <div style={{ position:'absolute', inset: 0, background:'#ffffff' }}>
        {/* top bar */}
        <div style={{
          height: 40, display:'flex', alignItems:'center', gap: 14, padding:'0 16px',
          background:'#1c1a16', color:'#f0ebdf', fontSize: 12,
        }}>
          <div style={{ width: 22, height: 22, borderRadius:'50%', background:'#f0ebdf', opacity:.88 }}/>
          <span style={{ fontSize: 12, fontFamily:'var(--mono)' }}>user / project</span>
          <span style={{ color:'#6b6252' }}>/</span>
          <span style={{ fontSize: 12 }}>Releases</span>
          <span style={{ marginLeft:'auto', fontSize: 11, color:'rgba(240,235,223,.6)', fontFamily:'var(--mono)' }}>main · v2.0.0-beta.3</span>
        </div>

        {/* page content — grey text because it's being "covered" */}
        <div style={{ padding:'36px 60px', color:'#1c1a16' }}>
          {blocks.map((b, i) => (
            <div key={i} style={{
              fontSize: b.size, fontWeight: b.weight, marginBottom: 4,
              lineHeight: b.h > b.size * 1.8 ? 1.55 : 1.4,
              color: b.muted ? '#6b6252' : '#1c1a16',
              width: b.w,
            }}>
              {b.en}
            </div>
          ))}
        </div>
      </div>

      {/* Single Lenz accent bar at top-left of overlay region — one marker, not N */}
      <div style={{
        position:'absolute', top: 90, left: 50, width: 4, height: 320,
        background:'linear-gradient(180deg, var(--lens), rgba(78,163,217,0))',
        borderRadius: 2, pointerEvents:'none',
      }}/>
      {/* The overlay layer — per-block translated cards positioned at original bboxes */}
      <div style={{ position:'absolute', inset: 0, pointerEvents:'none' }}>
        {blocks.map((b, i) => (
          <div key={i} style={{
            position:'absolute', left: b.x, top: b.y, width: b.w,
            // subtle edge-color fill — in production this samples original pixels.
            // here we approximate with warm paper tone + slight blur glow underneath.
            background:'#ffffff',
            boxShadow:`
              0 0 0 4px #ffffff,
              0 1px 0 rgba(78,163,217,.0)
            `,
            borderRadius: 3,
            padding:'0 1px',
            fontSize: b.size, fontWeight: b.weight,
            lineHeight: b.h > b.size * 1.8 ? 1.55 : 1.4,
            color: b.muted ? '#6b6252' : '#1c1a16',
            // 中文字体微调
            letterSpacing: b.size >= 18 ? '-.005em' : 0,
          }}>
            {b.zh}
          </div>
        ))}
      </div>

      {/* Floating control — bottom center */}
      <div style={{
        position:'absolute', bottom: 20, left:'50%', transform:'translateX(-50%)',
        padding:'6px 6px 6px 14px', borderRadius: 22,
        background:'rgba(26,24,20,.92)', backdropFilter:'blur(20px)',
        color:'#f0ebdf',
        display:'flex', alignItems:'center', gap: 12,
        boxShadow:'0 12px 28px rgba(0,0,0,.3), 0 0 0 0.5px rgba(255,255,255,.08)',
      }}>
        <LensMark size={18}/>
        <span style={{ fontSize: 12, fontWeight: 500 }}>覆盖中</span>
        <span style={{ fontSize: 11, color:'rgba(240,235,223,.55)', fontFamily:'var(--mono)' }}>6 区块 · EN → 中文</span>
        <span style={{ width: 1, height: 14, background:'rgba(255,255,255,.12)' }}/>
        <button style={{
          padding:'4px 10px', fontSize: 11, color:'#f0ebdf',
          background:'transparent', border:'none', cursor:'pointer',
          display:'flex', alignItems:'center', gap: 4, fontFamily:'var(--sf)',
        }}>
          <Icon name="swap" size={12}/>切换原文
        </button>
        <button style={{
          padding:'4px 11px', fontSize: 11, color:'#f0ebdf', fontWeight: 500,
          background:'rgba(255,255,255,.1)', border:'none', cursor:'pointer',
          borderRadius: 14, display:'flex', alignItems:'center', gap: 5, fontFamily:'var(--sf)',
        }}>
          <Kbd keys={['esc']} dark/> 关闭
        </button>
      </div>

      {/* top-right: language chip */}
      <div style={{
        position:'absolute', top: 54, right: 20,
        padding:'5px 10px', borderRadius: 7,
        background:'rgba(26,24,20,.9)', color:'#f0ebdf', fontSize: 11,
        display:'flex', alignItems:'center', gap: 6,
        fontFamily:'var(--mono)',
        boxShadow:'0 4px 12px rgba(0,0,0,.18)',
      }}>
        <span style={{ width: 5, height: 5, borderRadius:'50%', background:'var(--sage)', boxShadow:'0 0 5px var(--sage)' }}/>
        1.2s 完成翻译
      </div>
    </div>
  );
};

window.OverlayF3Artboard = OverlayF3Artboard;
