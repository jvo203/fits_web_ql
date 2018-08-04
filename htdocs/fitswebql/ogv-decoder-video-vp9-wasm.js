var OGVDecoderVideoVP9W = function(OGVDecoderVideoVP9W) {
  OGVDecoderVideoVP9W = OGVDecoderVideoVP9W || {};

var a;a||(a=typeof OGVDecoderVideoVP9W !== 'undefined' ? OGVDecoderVideoVP9W : {});var k=a;a.memoryLimit&&(a.TOTAL_MEMORY=k.memoryLimit);var q={},r;for(r in a)a.hasOwnProperty(r)&&(q[r]=a[r]);a.arguments=[];a.thisProgram="./this.program";a.quit=function(b,c){throw c;};a.preRun=[];a.postRun=[];var t=!1,u=!1,w=!1,x=!1;
if(a.ENVIRONMENT)if("WEB"===a.ENVIRONMENT)t=!0;else if("WORKER"===a.ENVIRONMENT)u=!0;else if("NODE"===a.ENVIRONMENT)w=!0;else if("SHELL"===a.ENVIRONMENT)x=!0;else throw Error("Module['ENVIRONMENT'] value is not valid. must be one of: WEB|WORKER|NODE|SHELL.");else t="object"===typeof window,u="function"===typeof importScripts,w="object"===typeof process&&"function"===typeof require&&!t&&!u,x=!t&&!w&&!u;
if(w){var y,z;a.read=function(b,c){y||(y=require("fs"));z||(z=require("path"));b=z.normalize(b);b=y.readFileSync(b);return c?b:b.toString()};a.readBinary=function(b){b=a.read(b,!0);b.buffer||(b=new Uint8Array(b));assert(b.buffer);return b};1<process.argv.length&&(a.thisProgram=process.argv[1].replace(/\\/g,"/"));a.arguments=process.argv.slice(2);process.on("unhandledRejection",function(){process.exit(1)});a.inspect=function(){return"[Emscripten Module object]"}}else if(x)"undefined"!=typeof read&&
(a.read=function(b){return read(b)}),a.readBinary=function(b){if("function"===typeof readbuffer)return new Uint8Array(readbuffer(b));b=read(b,"binary");assert("object"===typeof b);return b},"undefined"!=typeof scriptArgs?a.arguments=scriptArgs:"undefined"!=typeof arguments&&(a.arguments=arguments),"function"===typeof quit&&(a.quit=function(b){quit(b)});else if(t||u)a.read=function(b){var c=new XMLHttpRequest;c.open("GET",b,!1);c.send(null);return c.responseText},u&&(a.readBinary=function(b){var c=
new XMLHttpRequest;c.open("GET",b,!1);c.responseType="arraybuffer";c.send(null);return new Uint8Array(c.response)}),a.readAsync=function(b,c,d){var e=new XMLHttpRequest;e.open("GET",b,!0);e.responseType="arraybuffer";e.onload=function(){200==e.status||0==e.status&&e.response?c(e.response):d()};e.onerror=d;e.send(null)},a.setWindowTitle=function(b){document.title=b};else throw Error("not compiled for this environment");
a.print="undefined"!==typeof console?console.log.bind(console):"undefined"!==typeof print?print:null;a.printErr="undefined"!==typeof printErr?printErr:"undefined"!==typeof console&&console.warn.bind(console)||a.print;a.print=a.print;a.printErr=a.printErr;for(r in q)q.hasOwnProperty(r)&&(a[r]=q[r]);q=void 0;function aa(b){var c;c||(c=16);return Math.ceil(b/c)*c}var ba={"f64-rem":function(b,c){return b%c},"debugger":function(){debugger}},B=0;function assert(b,c){b||C("Assertion failed: "+c)}
"undefined"!==typeof TextDecoder&&new TextDecoder("utf8");"undefined"!==typeof TextDecoder&&new TextDecoder("utf-16le");function D(b,c){0<b%c&&(b+=c-b%c);return b}var buffer,ca,E,ja,F;function I(){a.HEAP8=ca=new Int8Array(buffer);a.HEAP16=ja=new Int16Array(buffer);a.HEAP32=F=new Int32Array(buffer);a.HEAPU8=E=new Uint8Array(buffer);a.HEAPU16=new Uint16Array(buffer);a.HEAPU32=new Uint32Array(buffer);a.HEAPF32=new Float32Array(buffer);a.HEAPF64=new Float64Array(buffer)}var J,K,L,O,P,Q,R,S;
J=K=O=P=Q=R=S=0;L=!1;a.reallocBuffer||(a.reallocBuffer=function(b){try{if(ArrayBuffer.g)var c=ArrayBuffer.g(buffer,b);else{var d=ca;c=new ArrayBuffer(b);(new Int8Array(c)).set(d)}}catch(e){return!1}return ka(c)?c:!1});var la;try{la=Function.prototype.call.bind(Object.getOwnPropertyDescriptor(ArrayBuffer.prototype,"byteLength").get),la(new ArrayBuffer(4))}catch(b){la=function(c){return c.byteLength}}var ma=a.TOTAL_STACK||5242880,T=a.TOTAL_MEMORY||16777216;
T<ma&&a.printErr("TOTAL_MEMORY should be larger than TOTAL_STACK, was "+T+"! (TOTAL_STACK="+ma+")");a.buffer?buffer=a.buffer:("object"===typeof WebAssembly&&"function"===typeof WebAssembly.Memory?(a.wasmMemory=new WebAssembly.Memory({initial:T/65536}),buffer=a.wasmMemory.buffer):buffer=new ArrayBuffer(T),a.buffer=buffer);I();F[0]=1668509029;ja[1]=25459;if(115!==E[2]||99!==E[3])throw"Runtime error: expected the system to be little-endian!";
function U(b){for(;0<b.length;){var c=b.shift();if("function"==typeof c)c();else{var d=c.h;"number"===typeof d?void 0===c.a?a.dynCall_v(d):a.dynCall_vi(d,c.a):d(void 0===c.a?null:c.a)}}}var na=[],oa=[],pa=[],qa=[],ra=[],sa=!1;function ta(){var b=a.preRun.shift();na.unshift(b)}var V=0,wa=null,W=null;a.preloadedImages={};a.preloadedAudios={};function X(b){return String.prototype.startsWith?b.startsWith("data:application/octet-stream;base64,"):0===b.indexOf("data:application/octet-stream;base64,")}
(function(){function b(){try{if(a.wasmBinary)return new Uint8Array(a.wasmBinary);if(a.readBinary)return a.readBinary(f);throw"on the web, we need the wasm binary to be preloaded and set on Module['wasmBinary']. emcc.py will do that for you when generating HTML (but not JS)";}catch(G){C(G)}}function c(){return a.wasmBinary||!t&&!u||"function"!==typeof fetch?new Promise(function(c){c(b())}):fetch(f,{credentials:"same-origin"}).then(function(b){if(!b.ok)throw"failed to load wasm binary file at '"+f+
"'";return b.arrayBuffer()}).catch(function(){return b()})}function d(b){function d(b){m=b.exports;if(m.memory){b=m.memory;var c=a.buffer;b.byteLength<c.byteLength&&a.printErr("the new buffer in mergeMemory is smaller than the previous one. in native wasm, we should grow memory here");c=new Int8Array(c);(new Int8Array(b)).set(c);a.buffer=buffer=b;I()}a.asm=m;a.usingWasm=!0;V--;a.monitorRunDependencies&&a.monitorRunDependencies(V);0==V&&(null!==wa&&(clearInterval(wa),wa=null),W&&(b=W,W=null,b()))}
function e(b){d(b.instance)}function g(b){c().then(function(b){return WebAssembly.instantiate(b,h)}).then(b).catch(function(b){a.printErr("failed to asynchronously prepare wasm: "+b);C(b)})}if("object"!==typeof WebAssembly)return a.printErr("no native wasm support detected"),!1;if(!(a.wasmMemory instanceof WebAssembly.Memory))return a.printErr("no native wasm Memory in use"),!1;b.memory=a.wasmMemory;h.global={NaN:NaN,Infinity:Infinity};h["global.Math"]=Math;h.env=b;V++;a.monitorRunDependencies&&a.monitorRunDependencies(V);
if(a.instantiateWasm)try{return a.instantiateWasm(h,d)}catch(M){return a.printErr("Module.instantiateWasm callback failed with error: "+M),!1}a.wasmBinary||"function"!==typeof WebAssembly.instantiateStreaming||X(f)||"function"!==typeof fetch?g(e):WebAssembly.instantiateStreaming(fetch(f,{credentials:"same-origin"}),h).then(e).catch(function(b){a.printErr("wasm streaming compile failed: "+b);a.printErr("falling back to ArrayBuffer instantiation");g(e)});return{}}var e="ogv-decoder-video-vp9-wasm.wast",
f="ogv-decoder-video-vp9-wasm.wasm",l="ogv-decoder-video-vp9-wasm.temp.asm.js";"function"===typeof a.locateFile&&(X(e)||(e=a.locateFile(e)),X(f)||(f=a.locateFile(f)),X(l)||(l=a.locateFile(l)));var h={global:null,env:null,asm2wasm:ba,parent:a},m=null;a.asmPreload=a.asm;var v=a.reallocBuffer;a.reallocBuffer=function(b){if("asmjs"===g)var c=v(b);else a:{b=D(b,a.usingWasm?65536:16777216);var d=a.buffer.byteLength;if(a.usingWasm)try{c=-1!==a.wasmMemory.grow((b-d)/65536)?a.buffer=a.wasmMemory.buffer:null;
break a}catch(fa){c=null;break a}c=void 0}return c};var g="";a.asm=function(b,c){if(!c.table){b=a.wasmTableSize;void 0===b&&(b=1024);var e=a.wasmMaxTableSize;c.table="object"===typeof WebAssembly&&"function"===typeof WebAssembly.Table?void 0!==e?new WebAssembly.Table({initial:b,maximum:e,element:"anyfunc"}):new WebAssembly.Table({initial:b,element:"anyfunc"}):Array(b);a.wasmTable=c.table}c.memoryBase||(c.memoryBase=a.STATIC_BASE);c.tableBase||(c.tableBase=0);c=d(c);assert(c,"no binaryen method succeeded.");
return c}})();J=1024;K=J+35408;oa.push();a.STATIC_BASE=J;a.STATIC_BUMP=35408;K+=16;assert(!L);var xa=K;K=K+4+15&-16;S=xa;O=P=aa(K);Q=O+ma;R=aa(Q);F[S>>2]=R;L=!0;a.wasmTableSize=166;a.wasmMaxTableSize=166;a.b={};
a.c={abort:C,enlargeMemory:function(){var b=a.usingWasm?65536:16777216,c=2147483648-b;if(F[S>>2]>c)return!1;var d=T;for(T=Math.max(T,16777216);T<F[S>>2];)536870912>=T?T=D(2*T,b):T=Math.min(D((3*T+2147483648)/4,b),c);b=a.reallocBuffer(T);if(!b||b.byteLength!=T)return T=d,!1;a.buffer=buffer=b;I();return!0},getTotalMemory:function(){return T},abortOnCannotGrowMemory:function(){C("Cannot enlarge memory arrays. Either (1) compile with  -s TOTAL_MEMORY=X  with X higher than the current value "+T+", (2) compile with  -s ALLOW_MEMORY_GROWTH=1  which allows increasing the size at runtime, or (3) if you want malloc to return NULL (0) instead of this abort, compile with  -s ABORTING_MALLOC=0 ")},
invoke_i:function(b){try{return a.dynCall_i(b)}catch(c){if("number"!==typeof c&&"longjmp"!==c)throw c;a.setThrew(1,0)}},invoke_ii:function(b,c){try{return a.dynCall_ii(b,c)}catch(d){if("number"!==typeof d&&"longjmp"!==d)throw d;a.setThrew(1,0)}},invoke_iii:function(b,c,d){try{return a.dynCall_iii(b,c,d)}catch(e){if("number"!==typeof e&&"longjmp"!==e)throw e;a.setThrew(1,0)}},invoke_iiiiii:function(b,c,d,e,f,l){try{return a.dynCall_iiiiii(b,c,d,e,f,l)}catch(h){if("number"!==typeof h&&"longjmp"!==h)throw h;
a.setThrew(1,0)}},invoke_v:function(b){try{a.dynCall_v(b)}catch(c){if("number"!==typeof c&&"longjmp"!==c)throw c;a.setThrew(1,0)}},invoke_vi:function(b,c){try{a.dynCall_vi(b,c)}catch(d){if("number"!==typeof d&&"longjmp"!==d)throw d;a.setThrew(1,0)}},invoke_viiii:function(b,c,d,e,f){try{a.dynCall_viiii(b,c,d,e,f)}catch(l){if("number"!==typeof l&&"longjmp"!==l)throw l;a.setThrew(1,0)}},invoke_viiiiii:function(b,c,d,e,f,l,h){try{a.dynCall_viiiiii(b,c,d,e,f,l,h)}catch(m){if("number"!==typeof m&&"longjmp"!==
m)throw m;a.setThrew(1,0)}},___setErrNo:function(b){a.___errno_location&&(F[a.___errno_location()>>2]=b);return b},_emscripten_memcpy_big:function(b,c,d){E.set(E.subarray(c,c+d),b);return b},_longjmp:function(b,c){a.setThrew(b,c||1);throw"longjmp";},_ogvjs_callback_frame:function(b,c,d,e,f,l,h,m,v,g,G,da,ea,fa,M,ua){var ha=a.HEAPU8,A=G+7&-8,N=da+7&-8,p=A*v/h,H=N*g/m,va=ea&-2,ia=fa&-2;h=va*v/h;v=ia*g/m;m=new Uint8Array(A*N);for(g=0;g<N;g++){var n=b+(g+ia)*c+ea;n=ha.subarray(n,n+A);m.set(n,A*g)}b=new Uint8Array(p*
H);for(g=0;g<H;g++)n=d+(g+v)*e+h,n=ha.subarray(n,n+p),b.set(n,p*g);d=new Uint8Array(p*H);for(g=0;g<H;g++)n=f+(g+v)*l+h,n=ha.subarray(n,n+p),d.set(n,p*g);f=a.videoFormat;G===f.cropWidth&&da===f.cropHeight&&(M=f.displayWidth,ua=f.displayHeight);a.frameBuffer={format:{width:A,height:N,chromaWidth:p,chromaHeight:H,cropLeft:ea-va,cropTop:fa-ia,cropWidth:G,cropHeight:da,displayWidth:M,displayHeight:ua},y:{bytes:m,stride:A},u:{bytes:b,stride:p},v:{bytes:d,stride:p}}},DYNAMICTOP_PTR:S,STACKTOP:P};
var ya=a.asm(a.b,a.c,buffer);a.asm=ya;var ka=a._emscripten_replace_memory=function(){return a.asm._emscripten_replace_memory.apply(null,arguments)};a._free=function(){return a.asm._free.apply(null,arguments)};a._malloc=function(){return a.asm._malloc.apply(null,arguments)};a._ogv_video_decoder_async=function(){return a.asm._ogv_video_decoder_async.apply(null,arguments)};a._ogv_video_decoder_destroy=function(){return a.asm._ogv_video_decoder_destroy.apply(null,arguments)};
a._ogv_video_decoder_init=function(){return a.asm._ogv_video_decoder_init.apply(null,arguments)};a._ogv_video_decoder_process_frame=function(){return a.asm._ogv_video_decoder_process_frame.apply(null,arguments)};a._ogv_video_decoder_process_header=function(){return a.asm._ogv_video_decoder_process_header.apply(null,arguments)};a.setThrew=function(){return a.asm.setThrew.apply(null,arguments)};a.dynCall_i=function(){return a.asm.dynCall_i.apply(null,arguments)};
a.dynCall_ii=function(){return a.asm.dynCall_ii.apply(null,arguments)};a.dynCall_iii=function(){return a.asm.dynCall_iii.apply(null,arguments)};a.dynCall_iiiiii=function(){return a.asm.dynCall_iiiiii.apply(null,arguments)};a.dynCall_v=function(){return a.asm.dynCall_v.apply(null,arguments)};a.dynCall_vi=function(){return a.asm.dynCall_vi.apply(null,arguments)};a.dynCall_viiii=function(){return a.asm.dynCall_viiii.apply(null,arguments)};
a.dynCall_viiiiii=function(){return a.asm.dynCall_viiiiii.apply(null,arguments)};a.asm=ya;a.then=function(b){if(a.calledRun)b(a);else{var c=a.onRuntimeInitialized;a.onRuntimeInitialized=function(){c&&c();b(a)}}return a};function Y(b){this.name="ExitStatus";this.message="Program terminated with exit("+b+")";this.status=b}Y.prototype=Error();Y.prototype.constructor=Y;W=function za(){a.calledRun||Aa();a.calledRun||(W=za)};
function Aa(){function b(){if(!a.calledRun&&(a.calledRun=!0,!B)){sa||(sa=!0,U(oa));U(pa);if(a.onRuntimeInitialized)a.onRuntimeInitialized();if(a.postRun)for("function"==typeof a.postRun&&(a.postRun=[a.postRun]);a.postRun.length;){var b=a.postRun.shift();ra.unshift(b)}U(ra)}}if(!(0<V)){if(a.preRun)for("function"==typeof a.preRun&&(a.preRun=[a.preRun]);a.preRun.length;)ta();U(na);0<V||a.calledRun||(a.setStatus?(a.setStatus("Running..."),setTimeout(function(){setTimeout(function(){a.setStatus("")},1);
b()},1)):b())}}a.run=Aa;a.exit=function(b,c){if(!c||!a.noExitRuntime||0!==b){if(!a.noExitRuntime&&(B=!0,P=void 0,U(qa),a.onExit))a.onExit(b);w&&process.exit(b);a.quit(b,new Y(b))}};function C(b){if(a.onAbort)a.onAbort(b);void 0!==b?(a.print(b),a.printErr(b),b=JSON.stringify(b)):b="";B=!0;throw"abort("+b+"). Build with -s ASSERTIONS=1 for more info.";}a.abort=C;if(a.preInit)for("function"==typeof a.preInit&&(a.preInit=[a.preInit]);0<a.preInit.length;)a.preInit.pop()();a.noExitRuntime=!0;Aa();
var Z,Ba,Ca;Ca="undefined"===typeof performance||"undefined"===typeof performance.now?Date.now:performance.now.bind(performance);function Da(b){var c=Ca();b=b();a.cpuTime+=Ca()-c;return b}a.loadedMetadata=!!k.videoFormat;a.videoFormat=k.videoFormat||null;a.frameBuffer=null;a.cpuTime=0;Object.defineProperty(a,"processing",{get:function(){return!1}});a.init=function(b){Da(function(){a._ogv_video_decoder_init()});b()};
a.processHeader=function(b,c){var d=Da(function(){var c=b.byteLength;Z&&Ba>=c||(Z&&a._free(Z),Ba=c,Z=a._malloc(Ba));var d=Z;a.HEAPU8.set(new Uint8Array(b),d);return a._ogv_video_decoder_process_header(d,c)});c(d)};a.f=[];a.processFrame=function(b,c){function d(b){a._free(l);c(b)}var e=a._ogv_video_decoder_async(),f=b.byteLength,l=a._malloc(f);e&&a.f.push(d);var h=Da(function(){a.HEAPU8.set(new Uint8Array(b),l);return a._ogv_video_decoder_process_frame(l,f)});e||d(h)};a.close=function(){};


  return OGVDecoderVideoVP9W;
};
if (typeof exports === 'object' && typeof module === 'object')
    module.exports = OGVDecoderVideoVP9W;
  else if (typeof define === 'function' && define['amd'])
    define([], function() { return OGVDecoderVideoVP9W; });
  else if (typeof exports === 'object')
    exports["OGVDecoderVideoVP9W"] = OGVDecoderVideoVP9W;
  