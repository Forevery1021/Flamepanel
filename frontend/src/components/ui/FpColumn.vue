<script lang="ts">
/**
 * FpColumn — OpenVue Column 的透明再导出。
 *
 * 为什么不能做成「包装组件」：OpenVue/PrimeVue DataTable 通过
 * `child.type.name === 'Column'`（HelperSet）收集列，并在渲染期通过
 * `vnode.type.extends.props[prop]`（getVNodeProp）读取列 prop 元数据。
 * 任何中间包装组件都会导致：① name 不匹配时列静默丢失（表格空白无报错）；
 * ② name 对齐后 extends.props 不存在直接 TypeError。
 * 因此封装层只能做透明再导出，保持 VNode type 即 Column 本身。
 * 后续如需统一样式，请在 FpTable 的 :deep() 或 pt 透传层实现。
 */
import Column from 'openvue/column'

export default Column
</script>
