<template>
  <div class="devui">
    <header class="devui-header">
      <div class="devui-title">
        <h1>FlamePanel · 组件预览（/dev/ui）</h1>
        <p class="devui-sub">
          开发者工具页：回归 Fp* 组件与设计令牌。生产环境无侧边栏入口，仅通过地址访问。
        </p>
      </div>
      <FpButton variant="ghost" icon="oi oi-external-link" @click="router.push('/dashboard')">
        返回面板
      </FpButton>
    </header>

    <FpTabs v-model="tab" :items="tabs" class="devui-tabs">
      <!-- ── 按钮 ── -->
      <template #buttons>
        <section class="devui-card">
          <h2>FpButton · 语义变体</h2>
          <div class="devui-row">
            <FpButton variant="primary">Primary</FpButton>
            <FpButton variant="secondary">Secondary</FpButton>
            <FpButton variant="danger">Danger</FpButton>
            <FpButton variant="success">Success</FpButton>
            <FpButton variant="warning">Warning</FpButton>
            <FpButton variant="ghost">Ghost</FpButton>
            <FpButton variant="link">Link</FpButton>
          </div>
          <h2>图标 / 加载 / 尺寸</h2>
          <div class="devui-row">
            <FpButton variant="primary" icon="oi oi-plus">新建</FpButton>
            <FpButton variant="secondary" icon="oi oi-refresh" icon-pos="right">刷新</FpButton>
            <FpButton variant="primary" :loading="true">加载中</FpButton>
            <FpButton variant="ghost" size="large">Large</FpButton>
            <FpButton variant="ghost" size="small">Small</FpButton>
          </div>
        </section>
      </template>

      <!-- ── 表格 ── -->
      <template #table>
        <section class="devui-card">
          <h2>FpTable + FpColumn + FpPagination</h2>
          <FpTable
            :rows="demoRows"
            :loading="false"
            :paginator="false"
            sortable
            sort-field="name"
            :sort-order="1"
            :empty-text="'无数据'"
          >
            <FpColumn field="id" header="ID" style="width: 80px" />
            <FpColumn field="name" header="名称" />
            <FpColumn field="role" header="角色">
              <template #body="{ data }">
                <FpTag :severity="roleSeverity(data.role)" :value="data.role" />
              </template>
            </FpColumn>
            <FpColumn :header="t('common.operation')" style="width: 120px">
              <template #body>
                <FpButton variant="link">编辑</FpButton>
              </template>
            </FpColumn>
          </FpTable>
          <div class="devui-row" style="margin-top: 12px">
            <FpPagination
              :first="0"
              :rows="10"
              :total="demoRows.length"
              :rows-per-page-options="[5, 10, 20]"
            />
          </div>
          <p class="devui-note">
            表格已支持：空态（FpEmpty）、加载态、虚拟滚动（virtual 模式）、客户端排序、窄屏横向滚动。
          </p>
        </section>
      </template>

      <!-- ── 弹窗 ── -->
      <template #modal>
        <section class="devui-card">
          <h2>FpModal + 表单控件</h2>
          <div class="devui-row">
            <FpButton variant="primary" @click="modalVisible = true">打开 FpModal</FpButton>
            <FpButton variant="ghost" @click="drawerVisible = true">打开 FpDrawer</FpButton>
          </div>
          <div class="devui-row" style="margin-top: 16px">
            <FpInput v-model="demoText" label="文本输入" style="max-width: 260px" />
            <FpSelect
              v-model="demoSelect"
              :options="demoOptions"
              option-label="label"
              option-value="value"
              label="下拉选择"
              style="max-width: 200px"
            />
            <FpNumber v-model="demoNumber" :min="1" :max="100" label="数字输入" style="max-width: 160px" />
          </div>
          <div class="devui-row" style="margin-top: 16px">
            <FpSwitch v-model="demoSwitch" />
            <FpCheckbox v-model="demoCheckbox" />
            <FpTag severity="success" :dot="true" value="运行中" />
            <FpTag severity="warning" value="警告" />
            <FpTag severity="danger" value="危险" />
            <FpTag severity="info" value="信息" />
          </div>
        </section>
      </template>

      <!-- ── 令牌 ── -->
      <template #tokens>
        <section class="devui-card">
          <h2>设计令牌（--fp-*，F2.1）</h2>
          <p class="devui-note">
            业务代码禁止写死色值，一律消费以下令牌。令牌随主题（明/暗 + 玻璃开关）自动切换。
          </p>
          <h3>Brand</h3>
          <div class="swatches">
            <div v-for="c in brandColors" :key="c" class="swatch" :style="{ background: `var(${c})` }">
              <span>{{ c }}</span>
            </div>
          </div>
          <h3>Semantic</h3>
          <div class="swatches">
            <div v-for="c in semanticColors" :key="c" class="swatch" :style="{ background: `var(${c})` }">
              <span>{{ c }}</span>
            </div>
          </div>
          <h3>Surface / Text</h3>
          <div class="swatches">
            <div
              v-for="c in surfaceColors"
              :key="c"
              class="swatch"
              :style="{ background: `var(${c})`, color: c.includes('bg') ? 'var(--fp-text-invert)' : '' }"
            >
              <span>{{ c }}</span>
            </div>
          </div>
          <h3>阴影 elevation</h3>
          <div class="devui-row">
            <div v-for="s in shadows" :key="s.name" class="shadow-demo" :style="{ boxShadow: `var(${s.var})` }">
              {{ s.name }}
            </div>
          </div>
        </section>
      </template>
    </FpTabs>

    <!-- 弹窗演示 -->
    <FpModal v-model="modalVisible" header="FpModal 演示" style="width: 480px">
      <p>这是 FpModal（重玻璃）。支持关闭按钮 / 遮罩关闭 / footer 插槽。</p>
      <div class="devui-row" style="margin-top: 12px">
        <FpInput v-model="demoText" label="示例字段" />
      </div>
      <template #footer>
        <FpButton variant="ghost" @click="modalVisible = false">取消</FpButton>
        <FpButton variant="primary" @click="modalVisible = false">确定</FpButton>
      </template>
    </FpModal>

    <FpDrawer v-model="drawerVisible" header="FpDrawer 演示" position="right" style="width: 360px">
      <p>侧边抽屉：适合编辑类面板。</p>
      <div class="devui-row" style="margin-top: 12px">
        <FpButton variant="primary" @click="drawerVisible = false">关闭</FpButton>
      </div>
    </FpDrawer>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import FpButton from '@/components/ui/FpButton.vue'
import FpTable from '@/components/ui/FpTable.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpPagination from '@/components/ui/FpPagination.vue'
import FpModal from '@/components/ui/FpModal.vue'
import FpDrawer from '@/components/ui/FpDrawer.vue'
import FpInput from '@/components/ui/FpInput.vue'
import FpSelect from '@/components/ui/FpSelect.vue'
import FpNumber from '@/components/ui/FpNumber.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'
import FpCheckbox from '@/components/ui/FpCheckbox.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpTabs from '@/components/ui/FpTabs.vue'
import type { FpTabItem } from '@/components/ui/FpTabs.vue'

const router = useRouter()
const { t } = useI18n()

const tab = ref('buttons')
const tabs: FpTabItem[] = [
  { value: 'buttons', label: '按钮' },
  { value: 'table', label: '表格' },
  { value: 'modal', label: '弹窗/表单' },
  { value: 'tokens', label: '设计令牌' },
]

const modalVisible = ref(false)
const drawerVisible = ref(false)
const demoText = ref('')
const demoSelect = ref('a')
const demoNumber = ref(10)
const demoSwitch = ref(true)
const demoCheckbox = ref(false)
const demoOptions = [
  { label: '选项 A', value: 'a' },
  { label: '选项 B', value: 'b' },
  { label: '选项 C', value: 'c' },
]

interface DemoRow {
  id: number
  name: string
  role: string
}
const demoRows: DemoRow[] = [
  { id: 1, name: 'admin', role: 'admin' },
  { id: 2, name: 'operator', role: 'operator' },
  { id: 3, name: 'viewer', role: 'viewer' },
  { id: 4, name: 'deploy-bot', role: 'operator' },
  { id: 5, name: 'auditor', role: 'viewer' },
]

function roleSeverity(role: string): 'success' | 'warning' | 'info' {
  if (role === 'admin') return 'success'
  if (role === 'operator') return 'warning'
  return 'info'
}

const brandColors = ['--fp-brand', '--fp-brand-strong', '--fp-brand-soft']
const semanticColors = [
  '--fp-success',
  '--fp-warning',
  '--fp-danger',
  '--fp-info',
  '--fp-success-soft',
  '--fp-warning-soft',
  '--fp-danger-soft',
  '--fp-info-soft',
]
const surfaceColors = [
  '--fp-bg-app',
  '--fp-bg-elevated',
  '--fp-bg-hover',
  '--fp-bg-sidebar',
  '--fp-bg-terminal',
  '--fp-border',
  '--fp-border-strong',
]
const shadows = [
  { name: 'xs', var: '--fp-shadow-xs' },
  { name: 'sm', var: '--fp-shadow-sm' },
  { name: 'md', var: '--fp-shadow-md' },
  { name: 'lg', var: '--fp-shadow-lg' },
  { name: 'brand', var: '--fp-shadow-brand' },
]
</script>

<style scoped>
.devui {
  max-width: 1080px;
  margin: 0 auto;
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-height: 100vh;
  background: var(--fp-bg-app);
  color: var(--fp-text-primary);
  font-family: var(--fp-font-sans);
}
.devui-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 20px 24px;
  border-radius: var(--fp-radius-md);
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
}
.devui-title h1 {
  margin: 0;
  font-size: 18px;
  font-weight: 700;
}
.devui-sub {
  margin: 6px 0 0;
  font-size: 13px;
  color: var(--fp-text-secondary);
}
.devui-tabs {
  width: 100%;
}
.devui-card {
  padding: 20px 24px;
  border-radius: var(--fp-radius-md);
  background: var(--fp-bg-elevated);
  border: 1px solid var(--fp-border);
  margin-bottom: 16px;
}
.devui-card h2 {
  margin: 0 0 14px;
  font-size: 15px;
  font-weight: 600;
}
.devui-card h3 {
  margin: 18px 0 10px;
  font-size: 13px;
  font-weight: 600;
  color: var(--fp-text-secondary);
}
.devui-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
}
.devui-note {
  margin-top: 10px;
  font-size: 12px;
  color: var(--fp-text-muted);
}
.swatches {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.swatch {
  width: 110px;
  height: 44px;
  border-radius: 8px;
  border: 1px solid var(--fp-border);
  display: flex;
  align-items: flex-end;
  padding: 4px 6px;
  font-size: 10px;
  color: var(--fp-text-invert);
  background-clip: padding-box;
}
.swatch span {
  background: rgb(0 0 0 / 0.35);
  padding: 1px 4px;
  border-radius: 4px;
  color: #fff;
}
.shadow-demo {
  width: 120px;
  height: 56px;
  border-radius: 8px;
  border: 1px solid var(--fp-border);
  background: var(--fp-bg-elevated);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
}
</style>
