// F2 — screen capture selection overlay (crosshair + dim mask)
const CaptureF2Artboard = ({ dark }) => {
  const W = 640, H = 400;
  // selection rect
  const sel = { x: 110, y: 96, w: 360, h: 190 };

  return (
    <div style={{
      width: W, height: H, position:'relative',
      borderRadius: 12, overflow:'hidden',
      boxShadow:'0 24px 48px -12px rgba(30,24,14,.2), 0 2px 6px rgba(30,24,14,.08)',
      fontFamily:'var(--sf)',
    }}>
      {/* Simulated desktop underneath */}
      <div style={{
        position:'absolute', inset: 0,
        background: `
          radial-gradient(120% 90% at 30% 20%, #6aa4d1, #3c6fa0 55%, #2a4e78 100%)`,
      }}>
        {/* fake menu bar */}
        <div style={{
          height: 22, background:'rgba(0,0,0,.25)', backdropFilter:'blur(10px)',
          display:'flex', alignItems:'center', gap: 14, padding:'0 12px',
          fontSize: 11, color:'rgba(255,255,255,.92)',
        }}>
          <span style={{fontWeight:600}}></span>
          <span>Safari</span>
          <span>File</span>
          <span>Edit</span>
          <span>View</span>
          <span style={{ marginLeft:'auto', display:'flex', alignItems:'center', gap: 10 }}>
            <LensMark size={13} />
            <span style={{fontFamily:'var(--mono)'}}>Wed 10:42</span>
          </span>
        </div>

        {/* fake browser window */}
        <div style={{
          position:'absolute', top: 46, left: 60, right: 60, bottom: 40,
          background:'#fff', borderRadius: 8,
          boxShadow:'0 30px 60px rgba(0,0,0,.3)', overflow:'hidden',
        }}>
          <div style={{ height: 36, background:'#f4f1ea', borderBottom:'0.5px solid #d8d1c2', display:'flex', alignItems:'center', gap: 8, padding:'0 12px' }}>
            <div style={{ display:'flex', gap: 5 }}>
              <i style={{ width:9, height:9, borderRadius:'50%', background:'#ff5f57'}}/>
              <i style={{ width:9, height:9, borderRadius:'50%', background:'#febc2e'}}/>
              <i style={{ width:9, height:9, borderRadius:'50%', background:'#28c840'}}/>
            </div>
            <div style={{
              flex: 1, height: 20, borderRadius: 4, background:'#fff',
              border:'0.5px solid #d8d1c2', fontSize: 10, color:'#6b6252',
              display:'flex', alignItems:'center', padding:'0 8px', fontFamily:'var(--mono)',
            }}>docs.example.com/guides/lifecycle</div>
          </div>
          <div style={{ padding: 16, color:'#1c1a16' }}>
            <div style={{ fontSize: 15, fontWeight:600, marginBottom: 6 }}>Component lifecycle</div>
            <div style={{ fontSize: 11, color:'#6b6252', marginBottom: 10 }}>
              Components mount, update, and unmount. Hooks fire in a predictable order, and side effects
              should be cleaned up before the next render cycle begins.
            </div>
            <div style={{ fontSize: 11, color:'#1c1a16', lineHeight: 1.6 }}>
              The useEffect hook runs after every commit. You can return a cleanup function to tear down
              subscriptions, timers, or event listeners.
            </div>
          </div>
        </div>
      </div>

      {/* Dim mask with hole */}
      <svg width={W} height={H} style={{ position:'absolute', inset: 0 }}>
        <defs>
          <mask id="hole">
            <rect width={W} height={H} fill="white" />
            <rect x={sel.x} y={sel.y} width={sel.w} height={sel.h} fill="black" />
          </mask>
        </defs>
        <rect width={W} height={H} fill="rgba(0,0,0,0.55)" mask="url(#hole)" />
        {/* selection border */}
        <rect x={sel.x - .5} y={sel.y - .5} width={sel.w + 1} height={sel.h + 1}
          fill="none" stroke="#4ea3d9" strokeWidth="1.5" />
        {/* corner handles */}
        {[[sel.x, sel.y],[sel.x+sel.w,sel.y],[sel.x,sel.y+sel.h],[sel.x+sel.w,sel.y+sel.h]].map(([x,y],i)=>(
          <rect key={i} x={x-3} y={y-3} width={6} height={6} fill="#fff" stroke="#4ea3d9" strokeWidth="1" />
        ))}
      </svg>

      {/* Crosshair guides */}
      <div style={{ position:'absolute', top: 0, bottom: 0, left: sel.x + sel.w, width: 1, background:'rgba(78,163,217,.3)' }} />
      <div style={{ position:'absolute', left: 0, right: 0, top: sel.y + sel.h, height: 1, background:'rgba(78,163,217,.3)' }} />

      {/* dimension readout (follows cursor) */}
      <div style={{
        position:'absolute', left: sel.x + sel.w + 12, top: sel.y + sel.h + 12,
        padding:'5px 9px', borderRadius: 6,
        background:'rgba(0,0,0,.8)', color:'#fff',
        fontSize: 11, fontFamily:'var(--mono)',
        display:'flex', gap: 8, alignItems:'center',
        boxShadow:'0 4px 12px rgba(0,0,0,.3)',
      }}>
        <span>{sel.w} × {sel.h}</span>
        <span style={{ width:1, height: 10, background:'rgba(255,255,255,.2)'}}/>
        <span style={{ opacity: .6 }}>{sel.x}, {sel.y}</span>
      </div>

      {/* top instruction pill */}
      <div style={{
        position:'absolute', top: 16, left:'50%', transform:'translateX(-50%)',
        padding:'6px 14px', borderRadius: 20,
        background:'rgba(0,0,0,.8)', color:'#fff',
        fontSize: 12, display:'flex', gap: 10, alignItems:'center',
        boxShadow:'0 8px 20px rgba(0,0,0,.3)',
      }}>
        <LensMark size={14}/>
        <span>拖动以框选区域</span>
        <span style={{ width:1, height: 12, background:'rgba(255,255,255,.2)'}}/>
        <span style={{ display:'flex', alignItems:'center', gap: 5, fontSize: 11, opacity: .75 }}>
          <Kbd keys={['esc']} /> 取消
        </span>
      </div>

      {/* Mode toggle at bottom */}
      <div style={{
        position:'absolute', bottom: 16, left:'50%', transform:'translateX(-50%)',
        padding: 4, borderRadius: 10,
        background:'rgba(0,0,0,.82)', backdropFilter:'blur(20px)',
        display:'flex', gap: 2, boxShadow:'0 10px 24px rgba(0,0,0,.35)',
      }}>
        <span style={{
          padding:'7px 14px', fontSize: 12, color:'#fff', fontWeight: 500,
          borderRadius: 7, background:'var(--lens)', display:'flex', gap: 7, alignItems:'center',
          boxShadow:'0 1px 2px rgba(0,0,0,.2), inset 0 1px 0 rgba(255,255,255,.18)',
        }}>
          <Icon name="capture" size={13}/>截图翻译
        </span>
        <span style={{
          padding:'7px 14px', fontSize: 12, color:'rgba(255,255,255,.7)',
          borderRadius: 7, display:'flex', gap: 7, alignItems:'center',
        }}>
          <Icon name="overlay" size={13}/>原图覆盖
        </span>
      </div>
    </div>
  );
};

window.CaptureF2Artboard = CaptureF2Artboard;
