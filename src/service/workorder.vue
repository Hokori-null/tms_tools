<template>
  <div class="page-container">
    <el-row :gutter="20">
      <!-- 左侧表单区域 -->
      <el-col :xs="24" :sm="24" :md="10">
        <el-card class="box-card">
          <template #header>
            <div class="card-header">
              <span>工单操作</span>
              <span v-if="username" class="username-display">欢迎, {{ username }}</span>
            </div>
          </template>
          <el-form label-width="80px" label-position="left">
            <el-form-item label="影院编码">
              <el-input v-model="form.n2nip" :disabled="isCreating" />
            </el-form-item>
            <el-form-item label="问题描述">
              <el-input v-model="form.messageQ" type="textarea" :rows="3" :disabled="isCreating" />
            </el-form-item>
            <el-form-item label="联系方式">
              <el-radio-group v-model="form.contact_way" :disabled="isCreating">
                <el-radio label="wx">微信</el-radio>
                <el-radio label="mobile">手机</el-radio>
              </el-radio-group>
            </el-form-item>
            <el-form-item label="号码">
              <el-input v-model="form.contact" :disabled="isCreating" />
            </el-form-item>
            <el-form-item label="工号">
              <el-input v-model="form.gh" :disabled="isCreating" />
            </el-form-item>
            <el-form-item label="工单ID">
              <el-input v-model="form.gdID" :disabled="isCreating" />
            </el-form-item>

            <el-form-item>
              <div class="form-buttons">
                <el-button type="primary" @click="createWorkOrder" :disabled="isCreating">创建工单</el-button>
                <el-button type="success" @click="feedbackSingleWorkOrder" :disabled="isCreating">反馈工单</el-button>
                <el-button type="danger" @click="closeSingleWorkOrder" :disabled="isCreating">关闭工单</el-button>
              </div>
            </el-form-item>
            <el-form-item>
              <div class="form-buttons">
                <el-button type="success" @click="feedbackTodaysOrders" :disabled="isCreating">反馈今日工单</el-button>
                <el-button type="danger" @click="closeTodaysOrders" :disabled="isCreating">关闭今日工单</el-button>
                <el-button type="warning" @click="feedbackAndCloseTodaysOrders" :disabled="isCreating">反馈并关闭今日工单</el-button>
              </div>
            </el-form-item>
            <el-form-item label="日志">
              <el-input type="textarea" :rows="4" v-model="log" disabled />
            </el-form-item>
          </el-form>
        </el-card>
      </el-col>

      <!-- 右侧工单列表区域 -->
      <el-col :xs="24" :sm="24" :md="14">
        <el-card class="box-card">
          <div class="action-bar">
            <el-button type="primary" @click="selectAll">全选</el-button>
            <el-select v-model="workOrderRange" placeholder="选择范围" size="small" @change="fetchWorkOrders" style="width: 120px;">
              <el-option label="今日工单" value="today"></el-option>
              <el-option label="本月工单" value="month"></el-option>
            </el-select>
            <el-select v-model="feedbackFilter" placeholder="反馈状态" size="small" clearable style="width: 120px;">
              <el-option label="已反馈" :value="1"></el-option>
              <el-option label="未反馈" :value="0"></el-option>
            </el-select>
            <el-select v-model="closeFilter" placeholder="关闭状态" size="small" clearable style="width: 120px;">
              <el-option label="已关闭" :value="1"></el-option>
              <el-option label="未关闭" :value="0"></el-option>
            </el-select>
            <div class="order-count">
              <span>工单总数: {{ filteredOrderList.length }}</span>
              <el-button type="primary" icon="Refresh" circle size="small" class="ml-2" @click="fetchWorkOrders"></el-button>
            </div>
            <div class="action-buttons">
              <el-button type="success" @click="feedbackSelected">反馈选中</el-button>
              <el-button type="warning" @click="closeSelected">结束选中</el-button>
            </div>
          </div>

          <div v-if="loading" class="loading-text">正在加载工单数据...</div>
          <div v-else-if="filteredOrderList.length === 0" class="loading-text">暂无工单数据</div>
          <div v-else class="order-list">
            <el-card v-for="order in filteredOrderList" :key="order.gdid" class="order-item">
              <template #header>
                <div class="order-header">
                  <span>工单ID: {{ order.gdid }}</span>
                  <span>影院编码: {{ order.n2n }}</span>
                </div>
              </template>
              <div class="order-body">
                <p><strong>问题描述:</strong> {{ order.messageq }}</p>
                <p><strong>处理结果:</strong> {{ order.messagea || '暂无处理结果' }}</p>
              </div>
              <div class="order-footer">
                <el-tag :type="order.iscreate === 0 ? 'success' : 'warning'">{{ order.iscreate === 0 ? '已创建' : '未创建' }}</el-tag>
                <el-tag :type="order.isfeedback === 1 ? 'success' : 'warning'">{{ order.isfeedback === 1 ? '已反馈' : '未反馈' }}</el-tag>
                <el-tag :type="order.isclose === 1 ? 'success' : 'warning'">{{ order.isclose === 1 ? '已关闭' : '未关闭' }}</el-tag>
                <el-checkbox :model-value="selectedOrders.has(order.gdid)" @change="toggleSelection(order.gdid)" />
              </div>
            </el-card>
          </div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup>
import { ref, reactive, onMounted, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';

const username = ref('');
const isCreating = ref(false);
const form = reactive({
  n2nip: '',
  messageQ: '',
  contact_way: 'mobile',
  contact: '',
  gh: '02',
  gdID: '',
  messageA: '',
});

const log = ref('');
const workOrderRange = ref('today'); // 'today' or 'month'
const feedbackFilter = ref(null); // 1 for feedbacked, 0 for not, null for all
const closeFilter = ref(null); // 1 for closed, 0 for not, null for all
const loading = ref(true);
const orderList = ref([]);
const selectedOrders = ref(new Set());

const filteredOrderList = computed(() => {
  return orderList.value.filter(order => {
    const feedbackMatch = feedbackFilter.value == null || order.isfeedback === feedbackFilter.value;
    const closeMatch = closeFilter.value == null || order.isclose === closeFilter.value;
    return feedbackMatch && closeMatch;
  });
});

async function fetchWorkOrders() {
  loading.value = true;
  try {
    const workOrderData = await invoke('get_workorders', { range: workOrderRange.value });
    orderList.value = workOrderData.orders;
  } catch (e) {
    log.value += `获取工单列表失败: ${e}\n`;
  } finally {
    loading.value = false;
  }
}

onMounted(async () => {
  try {
    const name = await invoke('get_dashboard_username');
    username.value = name;
  } catch (e) {
    log.value += `获取用户名失败: ${e}\n`;
  }
  await fetchWorkOrders();
});

async function createWorkOrder() {
  isCreating.value = true;
  log.value = '开始创建工单...\n';
  try {
    const massageQ = `${form.messageQ}${form.gh}`;
    const ticketPayload = {
      n2p: form.n2nip,
      massageQ: massageQ,
    };

    if (form.contact_way === 'wx') {
      ticketPayload.wx = form.contact;
      log.value += `调用 create_ticket...\n参数: n2p=${form.n2nip}, massageQ=${massageQ}, wx=${form.contact}\n`;
    } else if (form.contact_way === 'mobile') {
      ticketPayload.mobile = form.contact;
      log.value += `调用 create_ticket...\n参数: n2p=${form.n2nip}, massageQ=${massageQ}, mobile=${form.contact}\n`;
    }

    const code = await invoke('create_ticket_command', ticketPayload);

    log.value += `工单创建请求成功，返回代码: ${code}\n`;
    log.value += '正在将工单写入数据库...\n';

    await invoke('insert_workorder', {
      code: code,
      n2n: form.n2nip,
      q: form.messageQ,
      a: form.messageA,
    });

    log.value += '工单成功写入数据库。\n';
    form.gdID = code;
    form.n2nip = '';
    form.messageQ = '';
    form.contact = '';
    form.messageA = '';
    await fetchWorkOrders();
  } catch (error) {
    log.value += `创建工单过程中出错: ${error}\n`;
  } finally {
    isCreating.value = false;
  }
}

function toggleSelection(gdid) {
  if (selectedOrders.value.has(gdid)) {
    selectedOrders.value.delete(gdid);
  } else {
    selectedOrders.value.add(gdid);
  }
}

function selectAll() {
  const allIds = orderList.value.map(order => order.gdid);
  const allSelected = allIds.every(id => selectedOrders.value.has(id));

  if (allSelected) {
    selectedOrders.value.clear();
  } else {
    allIds.forEach(id => selectedOrders.value.add(id));
  }
}

async function feedbackSingleWorkOrder() {
  if (!form.gdID) {
    log.value += '请输入要反馈的工单ID\n';
    return;
  }
  isCreating.value = true;
  log.value += `开始反馈工单 ${form.gdID}...\n`;
  try {
    await invoke('feedback_workorder', { gdid: form.gdID });
    log.value += `工单 ${form.gdID} 反馈成功。\n`;
    await fetchWorkOrders();
  } catch (error) {
    log.value += `反馈工单 ${form.gdID} 出错: ${error}\n`;
  } finally {
    isCreating.value = false;
  }
}

async function closeSingleWorkOrder() {
  if (!form.gdID) {
    log.value += '请输入要关闭的工单ID\n';
    return;
  }
  isCreating.value = true;
  log.value += `开始关闭工单 ${form.gdID}...\n`;
  try {
    await invoke('close_workorder', { gdid: form.gdID });
    log.value += `工单 ${form.gdID} 关闭成功。\n`;
    await fetchWorkOrders();
  } catch (error) {
    log.value += `关闭工单出错: ${error}\n`;
  } finally {
    isCreating.value = false;
  }
}

async function feedbackSelected() {
  if (selectedOrders.value.size === 0) {
    log.value += '请先选择要反馈的工单。\n';
    return;
  }
  isCreating.value = true;
  log.value += `开始批量反馈 ${selectedOrders.value.size} 个工单...\n`;
  try {
    const gdids = Array.from(selectedOrders.value);
    const result = await invoke('feedback_selected_workorders', { gdids });
    log.value += `${result}\n`;
    await fetchWorkOrders();
    selectedOrders.value.clear();
  } catch (error) {
    log.value += `批量反馈出错: ${error}\n`;
  } finally {
    isCreating.value = false;
  }
}

async function closeSelected() {
  if (selectedOrders.value.size === 0) {
    log.value += '请先选择要关闭的工单。\n';
    return;
  }
  isCreating.value = true;
  log.value += `开始批量关闭 ${selectedOrders.value.size} 个工单...\n`;
  try {
    const gdids = Array.from(selectedOrders.value);
    const result = await invoke('close_selected_workorders', { gdids });
    log.value += `${result}\n`;
    await fetchWorkOrders();
    selectedOrders.value.clear();
  } catch (error) {
    log.value += `批量关闭出错: ${error}\n`;
  } finally {
    isCreating.value = false;
  }
}

async function feedbackTodaysOrders() {
    isCreating.value = true;
    log.value += `开始反馈今日所有工单...\n`;
    try {
        const result = await invoke('feedback_today_workorders');
        log.value += `${result}\n`;
        await fetchWorkOrders();
    } catch (error) {
        log.value += `反馈今日工单出错: ${error}\n`;
    } finally {
        isCreating.value = false;
    }
}

async function closeTodaysOrders() {
    isCreating.value = true;
    log.value += `开始关闭今日所有工单...\n`;
    try {
        const result = await invoke('close_today_workorders');
        log.value += `${result}\n`;
        await fetchWorkOrders();
    } catch (error) {
        log.value += `关闭今日工单出错: ${error}\n`;
    } finally {
        isCreating.value = false;
    }
}

async function feedbackAndCloseTodaysOrders() {
    isCreating.value = true;
    log.value += `开始反馈并关闭今日所有工单...\n`;
    try {
        const result = await invoke('feedback_and_close_today_workorders');
        log.value += `${result}\n`;
        await fetchWorkOrders();
    } catch (error) {
        log.value += `反馈并关闭今日工单出错: ${error}\n`;
    } finally {
        isCreating.value = false;
    }
}

</script>

<style scoped>
.page-container {
  padding: 20px;
  background-color: #f5f7fa;
  min-height: 100vh;
  box-sizing: border-box;
}

.box-card {
  height: 100%;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.username-display {
  font-size: 14px;
  color: #606266;
}

.form-buttons {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.form-buttons .el-button {
  width: auto;
}

.action-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 15px;
  align-items: center;
  margin-bottom: 20px;
}

.days-ago-container,
.order-count,
.action-buttons,
.auth-buttons {
  display: flex;
  align-items: center;
  gap: 8px;
}

.ml-2 {
  margin-left: 8px; /* Kept for compatibility if needed elsewhere */
}


.loading-text {
  padding: 40px;
  text-align: center;
  color: #95a5a6;
  font-size: 16px;
}

.order-list {
  max-height: calc(100vh - 350px); /* Adjust height dynamically */
  overflow-y: auto;
  padding-right: 10px; /* For scrollbar */
}

.order-item {
  margin-bottom: 15px;
  border-radius: 8px;
}

.order-header {
  display: flex;
  justify-content: space-between;
  font-weight: bold;
}

.order-body p {
  margin: 8px 0;
}

.order-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 15px;
  gap: 10px;
  flex-wrap: wrap;
}

/* Responsive adjustments */
@media (max-width: 768px) {
  .el-col {
    margin-bottom: 20px;
  }

  .action-bar {
    flex-direction: column;
    align-items: stretch;
    gap: 10px;
  }

  .action-bar > div {
    justify-content: space-between;
  }

  .order-list {
    max-height: none; /* Allow list to expand on mobile */
  }
}
</style>