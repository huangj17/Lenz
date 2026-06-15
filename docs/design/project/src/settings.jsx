// Settings window — sidebar + content panes
const SettingsArtboard = ({ dark, initialTab = 'services', initialSub = 'translate' }) => {
  const [tab, setTab] = React.useState(initialTab);

  const bg = dark ? '#1a1814' : '#faf8f4';
  const fg = dark ? '#f0ebdf' : '#1c1a16';
  const sub = dark ? 'rgba(240,235,223,.62)' : 'rgba(30,24,14,.58)';
  const border = dark ? 'rgba(255,255,255,.07)' : 'rgba(30,24,14,.07)';
  const cardBg = dark ? 'rgba(255,255,255,.035)' : '#fff';
  const fieldBg = dark ? 'rgba(0,0,0,.22)' : 'rgba(30,24,14,.03)';

  const nav = [
    { id:'general',  icon:'settings', label:'通用' },
    { id:'shortcuts',icon:'keyboard', label:'快捷键' },
    { id:'services', icon:'cpu',      label:'服务' },
    { id:'about',    icon:'info',     label:'关于' },
  ];

  return (
    <div className={`ln-win`} style={{
      width: 860, height: 580, color: fg,
      background: bg, display:'grid', gridTemplateColumns:'200px 1fr',
    }}>
      {/* Sidebar — vibrancy */}
      <div className={dark ? 'ln-vibrancy-dark' : 'ln-vibrancy-light'} style={{
        borderRight: `0.5px solid ${border}`,
        padding:'12px 10px', display:'flex', flexDirection:'column', gap: 2,
      }}>
        <div style={{ display:'flex', gap:8, alignItems:'center', padding:'4px 6px 14px' }}>
          <span style={{display:'flex', gap:6}}>
            <i style={{width:11,height:11,borderRadius:'50%',background:'#ff5f57'}}/>
            <i style={{width:11,height:11,borderRadius:'50%',background:'#febc2e'}}/>
            <i style={{width:11,height:11,borderRadius:'50%',background:'#28c840'}}/>
          </span>
        </div>
        <div style={{ padding:'0 8px 10px', display:'flex', alignItems:'center', gap:8 }}>
          <LensMark size={22} />
          <div style={{ fontSize:14, fontWeight:600, letterSpacing:'-.01em' }}>Lenz</div>
          <div style={{ marginLeft:'auto', fontSize:10, color: sub, fontFamily:'var(--mono)' }}>0.1.0</div>
        </div>

        {nav.map(n => (
          <button key={n.id} onClick={()=>setTab(n.id)} style={{
            display:'flex', alignItems:'center', gap: 9,
            padding:'7px 9px', border:'none', cursor:'pointer',
            borderRadius: 7, fontFamily:'var(--sf)', fontSize: 13,
            background: tab === n.id ? (dark ? 'rgba(78,163,217,.22)' : 'rgba(78,163,217,.18)') : 'transparent',
            color: tab === n.id ? 'var(--lens-dark)' : fg,
            fontWeight: 500,
            textAlign:'left',
          }}>
            <Icon name={n.icon} size={15} />
            <span>{n.label}</span>
          </button>
        ))}

        <div style={{ marginTop:'auto', padding:'8px', borderTop:`0.5px solid ${border}`, display:'flex', alignItems:'center', gap:8 }}>
          <div style={{ fontSize:11, color: sub }}>
            <div style={{display:'flex', alignItems:'center', gap:6}}>
              <span style={{width:6,height:6,borderRadius:'50%',background:'var(--sage)',boxShadow:'0 0 6px var(--sage)'}}/>
              <span>LM Studio 已连接</span>
            </div>
            <div style={{ fontFamily:'var(--mono)', fontSize:10, marginTop:2, color: sub }}>qwen2.5:7b · 127.0.0.1:1234</div>
          </div>
        </div>
      </div>

      {/* Content */}
      <div style={{ display:'flex', flexDirection:'column', minHeight:0 }}>
        <div style={{
          padding:'14px 24px', borderBottom:`0.5px solid ${border}`,
          display:'flex', alignItems:'center', justifyContent:'space-between',
          background: dark ? 'rgba(20,18,14,.4)' : 'rgba(250,248,244,.5)',
          backdropFilter:'blur(20px)',
        }}>
          <div style={{ fontSize: 15, fontWeight: 600 }}>{nav.find(n=>n.id===tab)?.label}</div>
          <div style={{ display:'flex', gap: 8, alignItems:'center' }}>
            <div style={{
              display:'flex', alignItems:'center', gap:6, padding:'4px 10px 4px 8px',
              border:`0.5px solid ${border}`, borderRadius: 6, fontSize: 12, color: sub,
              background: fieldBg,
            }}>
              <Icon name="search" size={13} />
              <span>搜索设置</span>
            </div>
          </div>
        </div>

        <div style={{ overflow:'hidden', flex: 1, padding:'22px 24px 24px', background: bg }}>
          {tab === 'services' && <ServicesPane {...{dark, fg, sub, border, cardBg, fieldBg, initialSub}} />}
          {tab === 'general'  && <GeneralPane {...{dark, fg, sub, border, cardBg, fieldBg}} />}
          {tab === 'shortcuts'&& <ShortcutsPane {...{dark, fg, sub, border, cardBg, fieldBg}} />}
          {tab === 'about'    && <AboutPane {...{dark, fg, sub, border, cardBg, fieldBg}} />}
        </div>
      </div>
    </div>
  );
};

// ——— Panes ———

const Section = ({ title, desc, children, border, sub }) => (
  <div style={{ marginBottom: 22 }}>
    <div style={{ fontSize: 11, letterSpacing:'.1em', textTransform:'uppercase', color: sub, marginBottom: 8, fontFamily:'var(--mono)', fontWeight:500 }}>{title}</div>
    {desc && <div style={{ fontSize: 12, color: sub, marginBottom: 10 }}>{desc}</div>}
    {children}
  </div>
);

const Row = ({ label, hint, children, border }) => (
  <div style={{
    display:'flex', alignItems:'center', justifyContent:'space-between',
    padding:'10px 14px', borderBottom: `0.5px solid ${border}`,
  }}>
    <div>
      <div style={{ fontSize: 13 }}>{label}</div>
      {hint && <div style={{ fontSize: 11, color: 'rgba(30,24,14,.5)', marginTop: 2 }}>{hint}</div>}
    </div>
    <div>{children}</div>
  </div>
);

const Toggle = ({ on, dark }) => (
  <div style={{
    width: 36, height: 21, borderRadius: 11,
    background: on ? 'var(--lens)' : (dark ? 'rgba(255,255,255,.15)' : 'rgba(30,24,14,.18)'),
    position:'relative', transition:'background .15s',
    boxShadow: on ? 'inset 0 1px 2px rgba(0,0,0,.1)' : 'inset 0 0.5px 2px rgba(0,0,0,.1)',
  }}>
    <span style={{
      position:'absolute', top: 2, left: on ? 17 : 2,
      width: 17, height: 17, borderRadius:'50%', background:'#fff',
      transition:'left .15s', boxShadow:'0 1px 3px rgba(0,0,0,.25)',
    }}/>
  </div>
);

const ServicesPane = ({ dark, fg, sub, border, cardBg, fieldBg, initialSub = 'translate' }) => {
  const [seg, setSeg] = React.useState(initialSub);
  const segs = [
    { id:'translate', icon:'cpu', label:'文本翻译' },
    { id:'ocr',       icon:'ocr', label:'OCR' },
  ];
  return (
    <div style={{ height:'100%', display:'flex', flexDirection:'column', minHeight: 0 }}>
      {/* Segmented control */}
      <div style={{
        display:'flex', gap: 3, padding: 3, marginBottom: 16,
        borderRadius: 9, background: fieldBg, alignSelf:'flex-start',
        border: `0.5px solid ${border}`,
      }}>
        {segs.map(s => (
          <button key={s.id} onClick={()=>setSeg(s.id)} style={{
            display:'flex', alignItems:'center', gap: 7,
            padding:'6px 16px', border:'none', cursor:'pointer',
            borderRadius: 6, fontFamily:'var(--sf)', fontSize: 12.5,
            background: seg === s.id ? (dark ? '#2a2721' : '#fff') : 'transparent',
            color: seg === s.id ? 'var(--lens-dark)' : sub,
            fontWeight: seg === s.id ? 600 : 500,
            boxShadow: seg === s.id ? '0 1px 3px rgba(0,0,0,.1)' : 'none',
          }}>
            <Icon name={s.icon} size={14}/>{s.label}
          </button>
        ))}
      </div>

      {/* Active sub-pane */}
      <div style={{ flex: 1, minHeight: 0, overflow:'auto' }}>
        {seg === 'translate'
          ? <EnginesPane {...{dark, fg, sub, border, cardBg, fieldBg}} />
          : <OcrPane {...{dark, fg, sub, border, cardBg, fieldBg}} />}
      </div>
    </div>
  );
};

const EnginesPane = ({ dark, fg, sub, border, cardBg, fieldBg }) => {
  const engines = [
    { name:'LM Studio（本地）', type:'OpenAI Compatible', model:'qwen2.5:7b-instruct', url:'http://127.0.0.1:1234/v1', status:'active', isDefault:true },
    { name:'Ollama',            type:'OpenAI Compatible', model:'llama3.1:8b',          url:'http://127.0.0.1:11434/v1', status:'idle', isDefault:false },
    { name:'DeepSeek',          type:'OpenAI Compatible', model:'deepseek-chat',        url:'https://api.deepseek.com', status:'idle', isDefault:false },
    { name:'Google 翻译',       type:'Traditional',       model:'—',                    url:'translate.googleapis.com', status:'idle', isDefault:false },
  ];

  return (
    <div style={{ height:'100%', display:'flex', flexDirection:'column' }}>
      <div style={{ display:'flex', justifyContent:'space-between', alignItems:'flex-end', marginBottom: 14 }}>
        <div style={{ fontSize: 12, color: sub, maxWidth: 440 }}>
          叠加多个引擎并设为默认。本地模型让每个请求都留在设备内；云端引擎会走代理设置。
        </div>
        <button style={{
          display:'flex', alignItems:'center', gap:6,
          padding:'6px 10px 6px 8px', border:'none', borderRadius: 6,
          background:'var(--lens)', color:'#fff', fontSize: 12, fontWeight: 500, cursor:'pointer',
          fontFamily:'var(--sf)',
          boxShadow:'0 1px 2px rgba(45,127,179,.3), inset 0 1px 0 rgba(255,255,255,.2)',
        }}>
          <Icon name="plus" size={13}/>添加服务
        </button>
      </div>

      <div style={{ display:'grid', gridTemplateColumns:'260px 1fr', gap: 14, flex: 1, minHeight: 0 }}>
        {/* List */}
        <div style={{ border:`0.5px solid ${border}`, borderRadius: 9, background: cardBg, overflow:'hidden' }}>
          {engines.map((e, i) => (
            <div key={e.name} style={{
            padding:'10px 12px 10px 14px', borderBottom: i < engines.length - 1 ? `0.5px solid ${border}` : 'none',
            background: i === 0 ? (dark ? 'rgba(78,163,217,.08)' : 'rgba(78,163,217,.06)') : 'transparent',
            borderLeft: i === 0 ? '3px solid var(--lens)' : '3px solid transparent',
            cursor:'pointer',
          }}>
              <div style={{ display:'flex', alignItems:'center', gap: 8 }}>
                <span style={{
                  width: 8, height: 8, borderRadius:'50%',
                  background: e.status === 'active' ? 'var(--sage)' : 'rgba(30,24,14,.2)',
                  boxShadow: e.status === 'active' ? '0 0 6px var(--sage)' : 'none',
                }}/>
                <div style={{ fontSize: 13, fontWeight: 500, flex: 1 }}>{e.name}</div>
                {e.isDefault && (
                  <span style={{
                    fontSize: 9.5, fontFamily:'var(--mono)', letterSpacing:'.08em',
                    textTransform:'uppercase', padding:'1px 6px', borderRadius: 4,
                    background:'var(--lens)', color:'#fff', fontWeight: 600,
                  }}>默认</span>
                )}
              </div>
              <div style={{ fontSize: 11, color: sub, marginTop: 3, fontFamily:'var(--mono)' }}>{e.model}</div>
            </div>
          ))}
        </div>

        {/* Editor */}
        <div style={{ border:`0.5px solid ${border}`, borderRadius: 9, background: cardBg, padding: 16, overflow:'auto' }}>
          <div style={{ display:'flex', alignItems:'center', gap: 10, marginBottom: 12 }}>
            <div style={{
              width: 26, height: 26, borderRadius: 6,
              background: 'linear-gradient(135deg, #4ea3d9, #2d7fb3)',
              color:'#fff', display:'flex', alignItems:'center', justifyContent:'center',
              fontFamily:'var(--mono)', fontSize: 11, fontWeight: 700,
            }}>LM</div>
            <div style={{ flex: 1 }}>
              <div style={{ fontSize: 14, fontWeight: 600 }}>LM Studio（本地）</div>
              <div style={{ fontSize: 11, color: sub, fontFamily:'var(--mono)' }}>OpenAI 兼容 · 流式输出</div>
            </div>
            <Toggle on={true} dark={dark} />
          </div>

          <Field label="API URL" value="http://127.0.0.1:1234/v1" {...{sub, border, fieldBg, fg}} mono />
          <Field label="API Key" value="sk-•••••••••••••••••••" {...{sub, border, fieldBg, fg}} mono />
          <Field label="模型" value="qwen2.5:7b-instruct" {...{sub, border, fieldBg, fg}} mono dropdown />

          <div style={{ marginTop: 10 }}>
            <div style={{ fontSize: 11, color: sub, marginBottom: 6, fontFamily:'var(--mono)', letterSpacing:'.06em', textTransform:'uppercase' }}>System Prompt</div>
            <div style={{
              border: `0.5px solid ${border}`, borderRadius: 7, padding: 10, background: fieldBg,
              fontSize: 12, color: fg, fontFamily:'var(--sf)', lineHeight: 1.5, minHeight: 66,
            }}>
              你是一位专业译者。请将文本从 <span style={{color:'var(--lens-dark)', fontFamily:'var(--mono)', background: dark ? 'rgba(78,163,217,.15)' : 'rgba(78,163,217,.12)', padding:'1px 4px', borderRadius: 3}}>{'{sourceLang}'}</span> 翻译为 <span style={{color:'var(--lens-dark)', fontFamily:'var(--mono)', background: dark ? 'rgba(78,163,217,.15)' : 'rgba(78,163,217,.12)', padding:'1px 4px', borderRadius: 3}}>{'{targetLang}'}</span>。保留原有格式与术语。
            </div>
          </div>

          <div style={{ marginTop: 16, display:'flex', gap: 8, alignItems:'center' }}>
            <button style={{ padding:'6px 12px', fontSize: 12, borderRadius: 6, border:`0.5px solid ${border}`, background:'transparent', color: fg, cursor:'pointer', fontFamily:'var(--sf)' }}>测试连接</button>
            <div style={{ display:'flex', alignItems:'center', gap: 5, fontSize: 11.5, color: 'var(--sage)' }}>
              <Icon name="check" size={13}/> 234 ms · 正常
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

const Field = ({ label, value, sub, border, fieldBg, fg, mono, dropdown }) => (
  <div style={{ marginBottom: 10 }}>
    <div style={{ fontSize: 11, color: sub, marginBottom: 4, fontFamily:'var(--mono)', letterSpacing:'.06em', textTransform:'uppercase' }}>{label}</div>
    <div style={{
      display:'flex', alignItems:'center',
      border: `0.5px solid ${border}`, borderRadius: 6, padding:'6px 10px',
      background: fieldBg, fontSize: 12.5, color: fg,
      fontFamily: mono ? 'var(--mono)' : 'var(--sf)',
    }}>
      <span style={{ flex: 1 }}>{value}</span>
      {dropdown && <Icon name="chevronDown" size={13} style={{ color: sub }}/>}
    </div>
  </div>
);

const GeneralPane = ({ dark, fg, sub, border, cardBg, fieldBg }) => (
  <div>
    <Section title="启动" sub={sub}>
      <div style={{ border:`0.5px solid ${border}`, borderRadius: 9, background: cardBg, overflow:'hidden' }}>
        <Row label="登录时启动" hint="在菜单栏静默启动 Lenz" border={border}><Toggle on={true} dark={dark}/></Row>
        <Row label="自动检查更新" hint="每天轮询一次 GitHub Releases" border={border}><Toggle on={true} dark={dark}/></Row>
        <Row label="在访达区显示图标" hint="关闭后仅驻留菜单栏" border={border}><Toggle on={false} dark={dark}/></Row>
      </div>
    </Section>

    <Section title="外观" sub={sub}>
      <div style={{ border:`0.5px solid ${border}`, borderRadius: 9, background: cardBg, overflow:'hidden' }}>
        <Row label="主题" border={border}>
          <div style={{ display:'flex', gap: 6, padding: 2, borderRadius: 7, background: fieldBg }}>
            {['跟随系统','浅色','深色'].map((t,i) => (
              <span key={t} style={{
                padding:'3px 10px', fontSize: 12, borderRadius: 5,
                background: i === 0 ? (dark ? '#2a2721' : '#fff') : 'transparent',
                boxShadow: i === 0 ? '0 1px 2px rgba(0,0,0,.08)' : 'none',
                fontWeight: i === 0 ? 500 : 400,
              }}>{t}</span>
            ))}
          </div>
        </Row>
        <Row label="界面语言" border={border}>
          <span style={{ fontSize: 12.5, display:'flex', alignItems:'center', gap:4 }}>简体中文 <Icon name="chevronDown" size={12}/></span>
        </Row>
      </div>
    </Section>

    <Section title="网络" sub={sub}>
      <div style={{ border:`0.5px solid ${border}`, borderRadius: 9, background: cardBg, overflow:'hidden' }}>
        <Row label="代理" hint="仅作用于云端引擎——本地请求不走代理" border={border}>
          <span style={{ fontSize: 12, fontFamily:'var(--mono)', color: sub }}>socks5://127.0.0.1:7890</span>
        </Row>
      </div>
    </Section>
  </div>
);

const ShortcutsPane = ({ dark, fg, sub, border, cardBg, fieldBg }) => {
  const items = [
    { label:'划词翻译',    keys:['⌥','D'], hint:'翻译光标处选中的文本' },
    { label:'截图翻译', keys:['⌥','S'], hint:'框选区域后 OCR 并显示译文' },
    { label:'原图覆盖翻译',  keys:['⌥','F'], hint:'将译文直接覆盖在原文像素上' },
    { label:'打开主窗口',  keys:['⌥','⌘','T'], hint:'调出设置页' },
  ];
  return (
    <div>
      <div style={{ border:`0.5px solid ${border}`, borderRadius: 9, background: cardBg, overflow:'hidden' }}>
        {items.map((it, i) => (
          <div key={it.label} className="ln-engrow" style={{
            display:'flex', alignItems:'center', padding:'14px 14px', gap: 12,
            borderBottom: i < items.length - 1 ? `0.5px solid ${border}` : 'none',
          }}>
            <div style={{
              width: 28, height: 28, borderRadius: 7,
              background: dark ? 'rgba(78,163,217,.14)' : 'rgba(78,163,217,.1)',
              color:'var(--lens-dark)',
              display:'flex', alignItems:'center', justifyContent:'center',
            }}>
              <Icon name={['selection','capture','overlay','settings'][i]} size={15}/>
            </div>
            <div style={{ flex: 1 }}>
              <div style={{ fontSize: 13, fontWeight: 500 }}>{it.label}</div>
              <div style={{ fontSize: 11, color: sub, marginTop: 2 }}>{it.hint}</div>
            </div>
            <Kbd keys={it.keys} dark={dark} />
            <button className="ln-change-btn" style={{
              padding:'4px 8px', fontSize: 11, border:`0.5px solid ${border}`,
              background:'transparent', color: sub, cursor:'pointer', borderRadius: 5,
              display:'flex', gap: 4, alignItems:'center', fontFamily:'var(--sf)',
            }}>
              <Icon name="settings" size={11}/>修改
            </button>
          </div>
        ))}
      </div>

      <div style={{ marginTop: 14, padding: 12, borderRadius: 8, border:`0.5px dashed ${border}`, fontSize: 12, color: sub, display:'flex', alignItems:'flex-start', gap: 10 }}>
        <Icon name="info" size={14} style={{ marginTop: 1 }}/>
        <div>划词翻译需要 macOS 的<b style={{color: fg, fontWeight: 500}}>辅助功能</b>权限；原图覆盖需要<b style={{color: fg, fontWeight: 500}}>屏幕录制</b>权限。两者都在首次使用时请求。</div>
      </div>
    </div>
  );
};

const OcrPane = ({ dark, fg, sub, border, cardBg, fieldBg }) => (
  <div>
    <Section title="引擎" sub={sub}>
      <div style={{ border:`0.5px solid ${border}`, borderRadius: 9, background: cardBg, overflow:'hidden' }}>
        <Row label="OCR 后端" hint="macOS Vision——无需下载" border={border}>
          <span style={{ fontSize: 12.5, display:'flex', alignItems:'center', gap:4 }}>Apple Vision <Icon name="chevronDown" size={12}/></span>
        </Row>
        <Row label="识别精度" hint="“精准”约慢 2 倍，但能处理小字" border={border}>
          <div style={{ display:'flex', gap: 6, padding: 2, borderRadius: 7, background: fieldBg }}>
            {['快速','精准'].map((t,i)=>(
              <span key={t} style={{padding:'3px 10px', fontSize: 12, borderRadius: 5,
                background: i===1 ? (dark ? '#2a2721' : '#fff') : 'transparent',
                boxShadow: i===1 ? '0 1px 2px rgba(0,0,0,.08)' : 'none',
                fontWeight: i===1 ? 500 : 400,
              }}>{t}</span>
            ))}
          </div>
        </Row>
        <Row label="自动检测语言" border={border}><Toggle on={true} dark={dark}/></Row>
      </div>
    </Section>
    <Section title="识别语言" desc="Vision 自动检测时优先考虑这些语言。拖动可排序。" sub={sub}>
      <div style={{ display:'flex', gap: 6, flexWrap:'wrap' }}>
        {['English','简体中文','日本語','한국어','Français','Deutsch','Español'].map((l,i)=>(
          <span key={l} style={{
            display:'inline-flex', alignItems:'center', gap: 6,
            padding:'5px 10px', borderRadius: 6,
            background: i < 3 ? (dark ? 'rgba(78,163,217,.18)' : 'rgba(78,163,217,.12)') : fieldBg,
            color: i < 3 ? 'var(--lens-dark)' : fg,
            fontSize: 12, fontWeight: i < 3 ? 500 : 400,
            border: `0.5px solid ${i < 3 ? 'rgba(78,163,217,.3)' : border}`,
          }}>
            <Icon name="drag" size={10} style={{opacity: .4}}/>{l}
          </span>
        ))}
      </div>
    </Section>
  </div>
);

const AboutPane = ({ dark, fg, sub, border, cardBg, fieldBg }) => (
  <div style={{ display:'flex', gap: 28, alignItems:'flex-start' }}>
    <LensMark size={88} />
    <div>
      <div style={{ fontSize: 28, fontWeight: 600, letterSpacing:'-.02em' }}>Lenz <span style={{ fontSize: 14, color: sub, fontFamily:'var(--mono)', fontWeight: 400, marginLeft: 8 }}>v0.1.0（测试版）</span></div>
      <div style={{ fontSize: 13, color: sub, marginTop: 4 }}>开源、跨平台的截图翻译工具。</div>
      <div style={{ fontSize: 13, color: sub, marginTop: 1, lineHeight: 1.5 }}>MIT 许可证 · Tauri 2.0 · Apple Vision OCR</div>

      <div style={{ display:'flex', gap: 8, marginTop: 16 }}>
        <button style={{ padding:'6px 12px', fontSize: 12, borderRadius: 6, border:'none', background:'var(--warm-700)', color:'var(--warm-white)', cursor:'pointer', display:'flex', gap:6, alignItems:'center' }}>
          <Icon name="external" size={12}/>GitHub
        </button>
        <button style={{ padding:'6px 12px', fontSize: 12, borderRadius: 6, border: `0.5px solid ${border}`, background:'transparent', color: fg, cursor:'pointer' }}>更新日志</button>
        <button style={{ padding:'6px 12px', fontSize: 12, borderRadius: 6, border: `0.5px solid ${border}`, background:'transparent', color: fg, cursor:'pointer' }}>检查更新</button>
      </div>

      <div style={{ marginTop: 26, padding: 14, border: `0.5px solid ${border}`, borderRadius: 9, background: cardBg, fontSize: 12, color: sub, maxWidth: 440, lineHeight: 1.55 }}>
        Lenz 完全运行在你的设备上。不上传任何数据、不采集埋点、无需账号。经由云端引擎（DeepSeek、Google 翻译等）的翻译仅发送给对应服务商。
      </div>
    </div>
  </div>
);

window.SettingsArtboard = SettingsArtboard;
