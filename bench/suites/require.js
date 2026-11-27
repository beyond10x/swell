// A CommonJS require for shells without one: bench/run.sh registers each module of
// are-we-fast-yet as benchModules['<name>'] = function (exports, require, module) { ... }.
var benchModules = {};
var benchModuleCache = {};
function require(name) {
  var key = name.replace(/^\.\//, '');
  if (benchModuleCache[key]) {
    return benchModuleCache[key].exports;
  }
  if (typeof benchModules[key] !== 'function') {
    throw new Error('no module ' + key);
  }
  var module = { exports: {} };
  benchModuleCache[key] = module;
  benchModules[key](module.exports, require, module);
  return module.exports;
}
