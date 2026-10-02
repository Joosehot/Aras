//! Oras output: an OBJ of the mannequin and the settled garment, and a
//! single-file 3D view (three.js) to turn them in a browser. `#back`, `#side`, `#left` or `#three` in the
//! address turns the view (for screenshots), `-bare` after it (`#side-bare`)
//! hides the garment; the g key toggles it.

use super::Outcome;
use std::fmt::Write as _;

fn flat3(v: &[[f64; 3]]) -> String {
    let mut s = String::with_capacity(v.len() * 24);
    for (i, p) in v.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        let _ = write!(s, "{:.1},{:.1},{:.1}", p[0], p[1], p[2]);
    }
    s
}

fn flat_t(t: &[[usize; 3]]) -> String {
    t.iter().map(|x| format!("{},{},{}", x[0], x[1], x[2])).collect::<Vec<_>>().join(",")
}

pub fn obj(o: &Outcome) -> String {
    let mut s = String::from("# Oras: mannequin (MakeHuman base mesh, CC0) and the settled garment, mm\no mannequin\n");
    for p in &o.body.v {
        let _ = writeln!(s, "v {:.2} {:.2} {:.2}", p[0], p[1], p[2]);
    }
    for t in &o.body.t {
        let _ = writeln!(s, "f {} {} {}", t[0] + 1, t[1] + 1, t[2] + 1);
    }
    let base = o.body.v.len();
    s.push_str("o garment\n");
    for p in &o.cloth.pos {
        let _ = writeln!(s, "v {:.2} {:.2} {:.2}", p[0], p[1], p[2]);
    }
    for t in &o.cloth.tri {
        let _ = writeln!(s, "f {} {} {}", t[0] + 1 + base, t[1] + 1 + base, t[2] + 1 + base);
    }
    s
}

/// `color` is the garment's CSS colour; `shine` 0..1 (satin high, velvet low).
pub fn html(o: &Outcome, title: &str, color: &str, shine: f64) -> String {
    PAGE.replace("__TITLE__", title)
        .replace("__BODY_V__", &flat3(&o.body.v))
        .replace("__BODY_T__", &flat_t(&o.body.t))
        .replace("__CLOTH_V__", &flat3(&o.cloth.pos))
        .replace("__CLOTH_T__", &flat_t(&o.cloth.tri))
        .replace("__COLOR__", color)
        .replace("__ROUGH__", &format!("{:.2}", 0.9 - 0.65 * shine))
        .replace("__REPORT__", &o.text.replace('&', "&amp;").replace('<', "&lt;"))
}

const PAGE: &str = r##"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>__TITLE__</title>
<style>
:root{--bg:#eceae6;--ink:#1d1c1a;--muted:#6b6862}
@media (prefers-color-scheme:dark){:root{--bg:#1a1918;--ink:#ecebe7;--muted:#a3a09a}}
html,body{margin:0;height:100%;background:var(--bg);color:var(--ink);font:13px/1.45 system-ui,Segoe UI,sans-serif;overflow:hidden}
canvas{display:block;width:100%;height:100%;touch-action:none}
#r{position:absolute;left:12px;bottom:10px;right:12px;white-space:pre-wrap;color:var(--muted);pointer-events:none}
#t{position:absolute;left:12px;top:10px;font-weight:600}
</style></head><body><div id="t">__TITLE__</div><pre id="r">__REPORT__</pre>
<script src="https://cdnjs.cloudflare.com/ajax/libs/three.js/r128/three.min.js"></script>
<script>
const R=new THREE.WebGLRenderer({antialias:true,preserveDrawingBuffer:true});R.setPixelRatio(devicePixelRatio);document.body.appendChild(R.domElement);
R.setClearColor(new THREE.Color(getComputedStyle(document.documentElement).getPropertyValue('--bg').trim()));
const S=new THREE.Scene(),C=new THREE.PerspectiveCamera(30,1,10,20000);
S.add(new THREE.HemisphereLight(0xffffff,0x444444,0.75));
const K=new THREE.DirectionalLight(0xffffff,0.8);K.position.set(600,1800,1500);S.add(K);
const F=new THREE.DirectionalLight(0xffffff,0.35);F.position.set(-900,900,-1200);S.add(F);
function mesh(v,t,mat){const g=new THREE.BufferGeometry();g.setAttribute('position',new THREE.Float32BufferAttribute(v,3));g.setIndex(t);g.computeVertexNormals();const m=new THREE.Mesh(g,mat);S.add(m);return m}
mesh([__BODY_V__],[__BODY_T__],new THREE.MeshStandardMaterial({color:0xd9d4cc,roughness:0.85}));
const cloth=mesh([__CLOTH_V__],[__CLOTH_T__],new THREE.MeshStandardMaterial({color:'__COLOR__',roughness:__ROUGH__,metalness:0.05,side:THREE.DoubleSide}));
const box=new THREE.Box3().setFromObject(S),ctr=box.getCenter(new THREE.Vector3());ctr.y=box.max.y*0.62;
const H=location.hash.slice(1).split('-');cloth.visible=!H.includes('bare');
addEventListener('keydown',e=>{if(e.key==='g'){cloth.visible=!cloth.visible;place()}});
let th={back:Math.PI,side:Math.PI/2,left:-Math.PI/2,three:0.6}[H[0]]||0,ph=1.45,rad=3600;
function place(){C.position.set(ctr.x+rad*Math.sin(ph)*Math.sin(th),ctr.y+rad*Math.cos(ph),ctr.z+rad*Math.sin(ph)*Math.cos(th));C.lookAt(ctr);R.render(S,C)}
function size(){R.setSize(innerWidth,innerHeight);C.aspect=innerWidth/innerHeight;C.updateProjectionMatrix();place()}
let d=null;R.domElement.onpointerdown=e=>{d=[e.clientX,e.clientY];R.domElement.setPointerCapture(e.pointerId)};R.domElement.onpointerup=()=>d=null;
R.domElement.onpointermove=e=>{if(!d)return;th-=(e.clientX-d[0])*0.01;ph=Math.min(3.0,Math.max(0.2,ph-(e.clientY-d[1])*0.01));d=[e.clientX,e.clientY];place()};
R.domElement.onwheel=e=>{e.preventDefault();rad*=Math.exp(e.deltaY*0.001);place()};
addEventListener('resize',size);size();
</script></body></html>"##;
