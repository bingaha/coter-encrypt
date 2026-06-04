<script setup>
import { computed, inject, onBeforeUnmount, ref } from 'vue'
import {
 NButton,
 NEmpty,
 NIcon,
 NInput,
 NSelect,
 NText,
 NTooltip,
 useMessage
} from 'naive-ui'
import {
 AddOutline,
 ArrowBackOutline,
 ClipboardOutline,
 CopyOutline,
 OpenOutline,
 MoonOutline,
 SearchOutline,
 SunnyOutline,
 TrashOutline
} from '@vicons/ionicons5'
import { useConfigStore } from '@/store'
import { invokeApi } from '@/api/tauriClient'

const configStore = useConfigStore()
const message = useMessage()

const isDarkMode = inject('isDarkMode', computed(() => configStore.isDarkMode))
const toggleTheme = inject('toggleTheme', () => configStore.toggleTheme())

const selectedCity = ref(null)
const selectedApp = ref(null)
const selectedBusinessType = ref(null)
const selectedAuditTag = ref(null)
const copied = ref(false)
const openingCls = ref(false)
let copiedTimer = null

const CLS_SEARCH_BASE_URL = 'https://console.cloud.tencent.com/cls/search?time=now-5m,now&topicType=log&multiple=false&timeZone=browser&analysis=eyJ0eXBlIjoidGFibGUifQ&region=ap-guangzhou&topic_id=e1a31d83-3230-4658-a59c-4e046bec2752&queryBase64=待补充&disabledRanges=W10'

function createKeywordId() {
 if (globalThis.crypto?.randomUUID) {
 return globalThis.crypto.randomUUID()
 }

 return `${Date.now()}-${Math.random().toString(36).slice(2)}`
}

const keywords = ref([{ id: createKeywordId(), value: '' }])

const cityGroups = [
 {
 province: '北京',
 cities: [
 { label: '北京', value: 'beijing' }
 ]
 },
 {
 province: '上海',
 cities: [
 { label: '上海', value: 'shanghai' }
 ]
 },
 {
 province: '广东',
 cities: [
 { label: '广州', value: 'guangzhou' },
 { label: '深圳', value: 'shenzhen' },
 { label: '深圳（社保专项）', value: 'shenzhenszzf' },
 { label: '珠海', value: 'zhuhai' },
 { label: '佛山', value: 'foshan' },
 { label: '惠州', value: 'huizhou' },
 { label: '江门', value: 'jiangmen' },
 { label: '中山', value: 'zhongshan' },
 { label: '东莞', value: 'dongguan' },
 { label: '东莞智服', value: 'dongguanzhifu' },
 { label: '汕头', value: 'shantou' },
 { label: '河源', value: 'heyuan' },
 { label: '汕尾', value: 'shanwei' },
 { label: '肇庆', value: 'zhaoqing' },
 { label: '阳江', value: 'yangjiang' },
 { label: '云浮', value: 'yunfu' },
 { label: '茂名', value: 'maoming' },
 { label: '韶关', value: 'shaoguan' },
 { label: '清远', value: 'qingyuan' },
 { label: '梅州', value: 'meizhou' },
 { label: '潮州', value: 'chaozhou' },
 { label: '湛江', value: 'zhanjiang' },
 { label: '揭阳', value: 'jieyang' }
 ]
 },
 {
 province: '江苏',
 cities: [
 { label: '南京', value: 'nanjing' },
 { label: '无锡', value: 'wuxi' },
 { label: '苏州园区', value: 'suzhouyuanqu' },
 { label: '苏州市区', value: 'suzhou' },
 { label: '南通', value: 'nantong' },
 { label: '常州', value: 'changzhou' },
 { label: '常熟', value: 'changshu' },
 { label: '吴江', value: 'wujiang' },
 { label: '昆山', value: 'kunshan' },
 { label: '盐城', value: 'yancheng' },
 { label: '张家港', value: 'zhangjiakang' },
 { label: '淮安', value: 'huaian' },
 { label: '扬州', value: 'yangzhou' },
 { label: '溧阳', value: 'liyang' },
 { label: '镇江', value: 'zhenjiang' },
 { label: '徐州', value: 'xuzhou' },
 { label: '宜兴', value: 'yixing' },
 { label: '连云港', value: 'lianyungang' },
 { label: '太仓', value: 'taicang' }
 ]
 },
 {
 province: '浙江',
 cities: [
 { label: '杭州', value: 'hangzhou' },
 { label: '温州', value: 'wenzhou' },
 { label: '宁波', value: 'ningbo' },
 { label: '金华', value: 'jinhua' },
 { label: '绍兴', value: 'shaoxing' },
 { label: '嘉善', value: 'jiashan' },
 { label: '台州', value: 'taizhou' },
 { label: '温岭', value: 'wenling' },
 { label: '余姚', value: 'yuyao' },
 { label: '平湖', value: 'pinghu' },
 { label: '舟山', value: 'zhoushan' },
 { label: '嘉兴', value: 'jiaxing' },
 { label: '萧山', value: 'xiaoshan' },
 { label: '湖州', value: 'huzhou' },
 { label: '桐乡', value: 'tongxiang' },
 { label: '丽水', value: 'lishui' },
 { label: '义乌', value: 'yiwu' },
 { label: '衢州', value: 'quzhou' },
 { label: '海宁', value: 'haining' },
 { label: '乐清', value: 'leqing' },
 { label: '瑞安', value: 'ruian' },
 { label: '慈溪', value: 'cixi' },
 { label: '嵊州', value: 'shengzhoushi' },
 { label: '龙港', value: 'longgangshi' },
 { label: '东阳市', value: 'dongyang' },
 { label: '兰溪市', value: 'lanxishi' },
 { label: '永康市', value: 'yongkangshi' },
 { label: '江山市', value: 'jiangshanshi' },
 { label: '临海市', value: 'linhaishi' },
 { label: '诸暨市', value: 'zhuji' },
 { label: '义乌ss', value: 'yiwuss' },
 { label: '南安市', value: 'nananshi' },
 { label: '晋江市', value: 'jinjiang' }
 ]
 },
 {
 province: '福建',
 cities: [
 { label: '厦门', value: 'xiamen' },
 { label: '福州', value: 'fuzhou2' },
 { label: '泉州', value: 'quanzhou' },
 { label: '三明', value: 'sanming' },
 { label: '南平', value: 'nanping' },
 { label: '漳州', value: 'zhangzhou' },
 { label: '宁德', value: 'ningde' },
 { label: '莆田', value: 'putian' },
 { label: '龙岩', value: 'longyan' }
 ]
 },
 {
 province: '安徽',
 cities: [
 { label: '合肥', value: 'hefei' },
 { label: '芜湖', value: 'wuhu' },
 { label: '宿州', value: 'anhuisuzhou' },
 { label: '蚌埠', value: 'bengbu' },
 { label: '亳州', value: 'bozhou' },
 { label: '滁州', value: 'chuzhou' },
 { label: '六安', value: 'luan' },
 { label: '宣城', value: 'xuancheng' },
 { label: '阜阳', value: 'fuyang' },
 { label: '铜陵', value: 'tongling' },
 { label: '巢湖', value: 'chaohu' },
 { label: '马鞍山', value: 'maanshan' },
 { label: '淮北', value: 'huaibei' },
 { label: '池州', value: 'chizhou' },
 { label: '淮南', value: 'huainan' },
 { label: '安庆', value: 'anqing' },
 { label: '黄山', value: 'huangshan' }
 ]
 },
 {
 province: '山东',
 cities: [
 { label: '青岛', value: 'qingdao' },
 { label: '潍坊', value: 'weifang' },
 { label: '临沂', value: 'linyi' },
 { label: '菏泽', value: 'heze' },
 { label: '滨州', value: 'binzhou' },
 { label: '烟台', value: 'yantai' },
 { label: '东营', value: 'dongying' },
 { label: '济宁', value: 'jining' },
 { label: '威海', value: 'weihai' },
 { label: '济南', value: 'jinan' },
 { label: '日照', value: 'rizhao' },
 { label: '聊城', value: 'liaocheng' },
 { label: '莱西', value: 'laixishi' },
 { label: '泰安', value: 'taian' },
 { label: '淄博', value: 'zibo' }
 ]
 },
 {
 province: '湖北',
 cities: [
 { label: '武汉', value: 'wuhan' },
 { label: '宜昌', value: 'yichang' },
 { label: '黄石', value: 'huangshi' },
 { label: '孝感', value: 'xiaogan' },
 { label: '仙桃', value: 'xiantao' },
 { label: '荆州', value: 'jingzhou' },
 { label: '随州', value: 'suizhou' },
 { label: '襄阳', value: 'xiangyang' },
 { label: '十堰', value: 'shiyan' },
 { label: '咸宁', value: 'xianning' },
 { label: '荆门', value: 'jingmen' },
 { label: '恩施', value: 'enshi' },
 { label: '潜江', value: 'qianjiang' },
 { label: '鄂州', value: 'ezhou' },
 { label: '黄冈', value: 'huanggang' }
 ]
 },
 {
 province: '四川',
 cities: [
 { label: '成都', value: 'chengdu' },
 { label: '成都智服', value: 'chengduzhifu' },
 { label: '成都（天府新区）', value: 'chengdutfxq' },
 { label: '成都（高新区）', value: 'chengdugxq' },
 { label: '成都（锦江区）', value: 'chengdu_jjq' },
 { label: '成都（龙泉驿区）', value: 'chengdulqyq' },
 { label: '成都市（双流区）', value: 'chengdusl' },
 { label: '成都ss', value: 'chengduss' },
 { label: '雅安', value: 'yaan' },
 { label: '乐山', value: 'leshan' },
 { label: '宜宾', value: 'yibin' },
 { label: '自贡', value: 'zigong' },
 { label: '南充', value: 'nanchong' },
 { label: '广安', value: 'guangan' },
 { label: '广元', value: 'guangyuan' },
 { label: '达州', value: 'dazhou' },
 { label: '内江', value: 'neijiang' },
 { label: '遂宁', value: 'suining' },
 { label: '德阳', value: 'deyang' },
 { label: '绵阳', value: 'mioanyang' },
 { label: '眉山', value: 'meishan' },
 { label: '泸州', value: 'luzhou' },
 { label: '西昌', value: 'xichang' }
 ]
 },
 {
 province: '江西',
 cities: [
 { label: '南昌', value: 'nanchang' },
 { label: '宜春', value: 'yichun' },
 { label: '九江', value: 'jiujiang' },
 { label: '赣州', value: 'ganzhou' },
 { label: '上饶', value: 'shangrao' },
 { label: '吉安', value: 'jian' },
 { label: '新余', value: 'xinyu' },
 { label: '萍乡', value: 'pingxiang' },
 { label: '抚州', value: 'fuzhou' },
 { label: '鹰潭', value: 'yingtan' },
 { label: '景德镇', value: 'jingdezhen' }
 ]
 },
 {
 province: '河南',
 cities: [
 { label: '郑州', value: 'zhengzhou' },
 { label: '洛阳', value: 'luoyang' },
 { label: '濮阳', value: 'puyang' },
 { label: '驻马店', value: 'zhumadian' },
 { label: '新乡', value: 'xinxiang' },
 { label: '周口', value: 'zhoukou' },
 { label: '开封', value: 'kaifeng' },
 { label: '商丘', value: 'shangqiu' },
 { label: '漯河', value: 'luohe' },
 { label: '南阳市', value: 'nanyang' },
 { label: '平顶山市', value: 'pingdingshan' },
 { label: '信阳市', value: 'xinyang' },
 { label: '许昌', value: 'xuchang' }
 ]
 },
 {
 province: '河北',
 cities: [
 { label: '石家庄', value: 'shijiazhuang' },
 { label: '唐山', value: 'tangshan' },
 { label: '沧州', value: 'cangzhou' },
 { label: '衡水', value: 'hengshui' },
 { label: '邢台', value: 'xingtai' },
 { label: '邯郸', value: 'handan1' },
 { label: '保定', value: 'baoding1' },
 { label: '廊坊', value: 'langfang' },
 { label: '承德', value: 'chengde' }
 ]
 },
 {
 province: '湖南',
 cities: [
 { label: '长沙', value: 'changsha' },
 { label: '邵阳', value: 'shaoyang' },
 { label: '郴州', value: 'chenzhou' },
 { label: '怀化', value: 'huaihua' },
 { label: '湘潭', value: 'xiangtan' },
 { label: '衡阳', value: 'hengyang' },
 { label: '岳阳', value: 'yueyang' },
 { label: '益阳', value: 'yiyang' },
 { label: '株洲', value: 'zhuzhou' },
 { label: '娄底', value: 'loudi' },
 { label: '常德', value: 'changde' }
 ]
 },
 {
 province: '重庆',
 cities: [
 { label: '重庆', value: 'chongqing' },
 { label: '重庆（政府）', value: 'chongqingzf' },
 { label: '重庆（网厅网申）', value: 'chongqingwkwsq' }
 ]
 },
 {
 province: '天津',
 cities: [
 { label: '天津', value: 'tianjinshi' }
 ]
 },
 {
 province: '辽宁',
 cities: [
 { label: '沈阳', value: 'shenyang' },
 { label: '大连', value: 'dalian' },
 { label: '抚顺', value: 'fushun' },
 { label: '大庆', value: 'daqing' }
 ]
 },
 {
 province: '吉林',
 cities: [
 { label: '长春', value: 'changchun' }
 ]
 },
 {
 province: '黑龙江',
 cities: [
 { label: '哈尔滨', value: 'haerbin' }
 ]
 },
 {
 province: '陕西',
 cities: [
 { label: '西安', value: 'xian' },
 { label: '咸阳', value: 'xianyang' },
 { label: '宝鸡', value: 'baoji' },
 { label: '安康', value: 'ankang' }
 ]
 },
 {
 province: '山西',
 cities: [
 { label: '太原', value: 'taiyuan' },
 { label: '晋中', value: 'jinzhong' },
 { label: '大同市', value: 'datong' },
 { label: '运城', value: 'yuncheng' }
 ]
 },
 {
 province: '内蒙古',
 cities: [
 { label: '呼和浩特市', value: 'huhehaote' },
 { label: '包头', value: 'baotou' },
 { label: '赤峰市', value: 'chifeng' }
 ]
 },
 {
 province: '海南',
 cities: [
 { label: '海南', value: 'haikou' },
 { label: '三亚', value: 'sanya' },
 { label: '澄迈', value: 'chengmai' },
 { label: '万宁市', value: 'wanning' },
 { label: '文昌', value: 'wenchang' },
 { label: '琼海', value: 'qionghai' },
 { label: '陵水', value: 'lingshui' }
 ]
 },
 {
 province: '广西',
 cities: [
 { label: '南宁', value: 'nanning' },
 { label: '柳州', value: 'liuzhou' },
 { label: '钦州', value: 'qinzhou' },
 { label: '梧州', value: 'wuzhou' },
 { label: '百色', value: 'baise' },
 { label: '玉林', value: 'yulin2' },
 { label: '桂林', value: 'guilin' },
 { label: '北海', value: 'beihai' },
 { label: '贵港', value: 'guigang' },
 { label: '防城港', value: 'fangchenggang' }
 ]
 },
 {
 province: '贵州',
 cities: [
 { label: '贵阳', value: 'guyang' },
 { label: '遵义', value: 'zunyi' },
 { label: '六盘水', value: 'liupanshui' },
 { label: '都匀', value: 'douyun' },
 { label: '铜仁', value: 'tongren' },
 { label: '凯里', value: 'kaili' },
 { label: '毕节', value: 'bijie' }
 ]
 },
 {
 province: '云南',
 cities: [
 { label: '昆明', value: 'kunming' },
 { label: '玉溪', value: 'yuxi' },
 { label: '大理', value: 'dali' }
 ]
 },
 {
 province: '甘肃',
 cities: [
 { label: '兰州', value: 'lanzhou' },
 { label: '武威', value: 'wuwei' },
 { label: '天水', value: 'tianshui' }
 ]
 },
 {
 province: '宁夏',
 cities: [
 { label: '银川', value: 'yinchuan' }
 ]
 },
 {
 province: '青海',
 cities: [
 { label: '西宁', value: 'xining' }
 ]
 },
 {
 province: '新疆',
 cities: [
 { label: '乌鲁木齐市', value: 'wulumuqjjjskfq' },
 { label: '昌吉', value: 'changji' }
 ]
 }
]

const businessTypeOptions = [
 { label: '社保凭证下载', value: 1 },
 { label: '无日期社保凭证下载', value: 2 },
 { label: '参保用户新增（社保增员） —— 医保增员、社保增员', value: 3 },
 { label: '参保用户新增凭证下载', value: 4 },
 { label: '参保用户停缴（社保减员）', value: 5 },
 { label: '参保用户补缴', value: 6 },
 { label: '新增退单', value: 7 },
 { label: '停缴退单', value: 8 },
 { label: '补缴退单', value: 9 },
 { label: '公积金新增', value: 10 },
 { label: '公积金停缴', value: 11 },
 { label: '公积金补缴', value: 12 },
 { label: '公积金新增退单', value: 13 },
 { label: '公积金停缴退单', value: 14 },
 { label: '公积金补缴退单', value: 15 },
 { label: '公积金凭证下载', value: 16 },
 { label: '社保同步员工花名册信息', value: 17 },
 { label: '公积金同步员工花名册信息', value: 18 },
 { label: '社保同步账单信息', value: 19 },
 { label: '公积金同步账单信息 —— 公积金台账下载', value: 20 },
 { label: '社保招工', value: 21 },
 { label: '社保退工', value: 22 },
 { label: '社保材料下载', value: 23 },
 { label: '劳动合同备案', value: 24 },
 { label: '劳动合同解除备案', value: 25 },
 { label: '安徽公积金汇缴提交', value: 26 },
 { label: '安徽公积金汇缴移除人员', value: 27 },
 { label: '安徽公积金补缴提交', value: 28 },
 { label: '安徽公积金补缴移出人员', value: 29 },
 { label: '下载转入劳动备案', value: 30 },
 { label: '下载转出劳动备案', value: 31 },
 { label: '招工退单', value: 32 },
 { label: '劳动合同备案退单', value: 33 },
 { label: '月度调基', value: 34 },
 { label: '生成合同', value: 35 },
 { label: '汇缴缴费申报', value: 36 },
 { label: '补缴缴费申报', value: 37 },
 { label: '汇缴缴费', value: 38 },
 { label: '补缴缴费', value: 39 },
 { label: '缴费工资申报与调整', value: 40 },
 { label: '汇补缴', value: 41 },
 { label: '封存转出', value: 42 },
 { label: '完税证明下载', value: 43 },
 { label: '截图下载', value: 44 },
 { label: '集中封存', value: 45 },
 { label: '登录', value: 'login' },
 { label: '获取短信验证码', value: 'sendSmsCode' }
]

const auditTagOptions = [
 { label: '实操任务', value: 0 },
 { label: '审核任务', value: 1 }
]

const appOptions = [
 { label: 'platform-crawler-service', value: 'platform-crawler-service' },
 { label: 'shebao-robot-service', value: 'shebao-robot-service' }
]

const cityOptions = computed(() => cityGroups.map(group => ({
 type: 'group',
 label: group.province,
 key: group.province,
 children: group.cities.map(city => ({
 ...city,
 province: group.province
 }))
})))

const cityFilter = (pattern, option) => {
 const keyword = pattern.trim().toLowerCase()

 if (!keyword) {
 return true
 }

 return [
 option.label,
 option.value,
 option.province
 ].some(value => String(value || '').toLowerCase().includes(keyword))
}

const selectedCityOption = computed(() => {
 for (const group of cityGroups) {
 const city = group.cities.find(item => item.value === selectedCity.value)
 if (city) {
 return {
 ...city,
 province: group.province
 }
 }
 }

 return null
})

const selectedAppOption = computed(() => {
 return appOptions.find(item => item.value === selectedApp.value) || null
})

const selectedBusinessTypeOption = computed(() => {
 return businessTypeOptions.find(item => item.value === selectedBusinessType.value) || null
})

const selectedAuditTagOption = computed(() => {
 return auditTagOptions.find(item => item.value === selectedAuditTag.value) || null
})

const taskRoute = computed(() => {
 const taskType = selectedBusinessType.value

 if (taskType === 'login') {
 return {
 label: '登录',
 prefix: 'success invoke login method, request param is',
 taskField: 'none'
 }
 }

 if (taskType === 'sendSmsCode') {
 return {
 label: '获取短信验证码',
 prefix: 'success invoke sendSmsCode method, request param is',
 taskField: 'none'
 }
 }

 if ([1, 2, 16].includes(taskType)) {
 return {
 label: '凭证下载',
 prefix: 'downloadUser task',
 taskField: 'type'
 }
 }

 if ([26, 27, 28, 29].includes(taskType)) {
 return {
 label: '安徽公积金',
 prefix: 'anHuiSubmit respResult',
 taskField: 'type'
 }
 }

 return {
 label: '常规任务',
 prefix: 'commonService defaultInsuredUser',
 taskField: 'taskType'
 }
})

const activeKeywords = computed(() => {
 return keywords.value
 .map(keyword => keyword.value.trim())
 .filter(Boolean)
})

const expressionParts = computed(() => {
 const parts = []

 if (selectedApp.value) {
 parts.push(`app:"${selectedApp.value}"`)
 }

 if (selectedBusinessType.value !== null) {
 parts.push(`log:'${taskRoute.value.prefix}'`)

 if (taskRoute.value.taskField === 'taskType') {
 parts.push(`log:'taskType":${selectedBusinessType.value}'`)
 } else if (taskRoute.value.taskField === 'type') {
 parts.push(`log:'"type":${selectedBusinessType.value}'`)
 }
 }

 if (selectedCity.value) {
 parts.push(`log:'areaCode":"${selectedCity.value}'`)
 }

 if (selectedAuditTag.value !== null) {
 parts.push(`log:'"auditTag":${selectedAuditTag.value}'`)
 }

 activeKeywords.value.forEach(keyword => {
 parts.push(`log:'${keyword}'`)
 })

 return parts
})

const expression = computed(() => expressionParts.value.join(' and '))
const hasExpression = computed(() => expression.value.length > 0)

const queryBase64 = computed(() => {
 if (!hasExpression.value) {
 return ''
 }

 const bytes = new TextEncoder().encode(expression.value)
 let binary = ''
 bytes.forEach(byte => {
 binary += String.fromCharCode(byte)
 })

 return btoa(binary)
})

const clsSearchUrl = computed(() => {
 if (!queryBase64.value) {
 return ''
 }

 return CLS_SEARCH_BASE_URL.replace('待补充', encodeURIComponent(queryBase64.value))
})

const addKeyword = () => {
 keywords.value.push({ id: createKeywordId(), value: '' })
}

const removeKeyword = (id) => {
 if (keywords.value.length === 1) {
 keywords.value = [{ id: createKeywordId(), value: '' }]
 return
 }

 keywords.value = keywords.value.filter(keyword => keyword.id !== id)
}

const handleCopy = async () => {
 if (!hasExpression.value) {
 message.warning('暂无可复制的表达式')
 return
 }

 try {
 await navigator.clipboard.writeText(expression.value)
 copied.value = true
 message.success('表达式已复制')

 if (copiedTimer) {
 clearTimeout(copiedTimer)
 }

 copiedTimer = window.setTimeout(() => {
 copied.value = false
 copiedTimer = null
 }, 1600)
 } catch (error) {
 message.error('复制失败')
 }
}

const handleOpenCls = async () => {
 if (!hasExpression.value) {
 message.warning('请先选择查询条件')
 return
 }

 openingCls.value = true
 try {
 await invokeApi('open_external_url', { url: clsSearchUrl.value })
 message.success('已打开腾讯云日志查询')
 } catch (error) {
 message.error(error?.message || '打开浏览器失败')
 } finally {
 openingCls.value = false
 }
}

const handleToggleTheme = () => {
 toggleTheme()
}

onBeforeUnmount(() => {
 if (copiedTimer) {
 clearTimeout(copiedTimer)
 }
})
</script>

<template>
 <main class="log-expression-page">
 <header class="page-header">
 <div class="header-left">
 <n-tooltip trigger="hover">
 <template #trigger>
 <router-link class="icon-link" to="/">
 <n-icon><ArrowBackOutline /></n-icon>
 </router-link>
 </template>
 返回首页
 </n-tooltip>

 <div class="title-mark">
 <n-icon :size="23"><SearchOutline /></n-icon>
 </div>

 <div class="title-copy">
 <h1>日志查询表达式生成器</h1>
 <n-text depth="3">按城市、业务类型和任务类型实时拼接日志查询条件</n-text>
 </div>
 </div>

 <n-tooltip trigger="hover">
 <template #trigger>
 <n-button quaternary circle @click="handleToggleTheme">
 <template #icon>
 <n-icon>
 <MoonOutline v-if="!isDarkMode" />
 <SunnyOutline v-else />
 </n-icon>
 </template>
 </n-button>
 </template>
 {{ isDarkMode ? '切换到亮色模式' : '切换到暗色模式' }}
 </n-tooltip>
 </header>

 <section class="page-shell">
 <div class="form-area">
 <section class="panel">
 <div class="panel-heading">
 <h2>查询条件</h2>
 </div>

 <div class="selector-grid">
 <label class="field-block">
 <span>App</span>
 <n-select
 v-model:value="selectedApp"
 :options="appOptions"
 placeholder="请选择 app"
 clearable
 />
 </label>

 <label class="field-block">
 <span>城市</span>
 <n-select
 v-model:value="selectedCity"
 :options="cityOptions"
 placeholder="请选择城市"
 clearable
 filterable
 :filter="cityFilter"
 />
 </label>

 <label class="field-block">
 <span>业务类型</span>
 <n-select
 v-model:value="selectedBusinessType"
 :options="businessTypeOptions"
 placeholder="请选择业务类型"
 clearable
 filterable
 />
 </label>

 <label class="field-block">
 <span>任务类型</span>
 <n-select
 v-model:value="selectedAuditTag"
 :options="auditTagOptions"
 placeholder="请选择任务类型"
 clearable
 />
 </label>
 </div>

 <div class="selection-summary">
 <div>
 <n-text depth="3">app</n-text>
 <strong>{{ selectedAppOption?.value || '未选择' }}</strong>
 </div>
 <div>
 <n-text depth="3">当前路径</n-text>
 <strong>{{ selectedBusinessType === null ? '未选择业务类型' : taskRoute.label }}</strong>
 </div>
 <div>
 <n-text depth="3">城市 code</n-text>
 <strong>{{ selectedCityOption?.value || '未选择' }}</strong>
 </div>
 <div>
 <n-text depth="3">taskType</n-text>
 <strong>{{ taskRoute.taskField === 'none' ? '不涉及' : (selectedBusinessTypeOption?.value ?? '未选择') }}</strong>
 </div>
 <div>
 <n-text depth="3">auditTag</n-text>
 <strong>{{ selectedAuditTagOption?.value ?? '未选择' }}</strong>
 </div>
 </div>
 </section>

 <section class="panel">
 <div class="panel-heading panel-heading-row">
 <h2>自定义关键词</h2>
 <n-button secondary size="small" @click="addKeyword">
 <template #icon>
 <n-icon><AddOutline /></n-icon>
 </template>
 添加关键词
 </n-button>
 </div>

 <div class="keyword-list">
 <div
 v-for="(keyword, index) in keywords"
 :key="keyword.id"
 class="keyword-row"
 >
 <n-input
 v-model:value="keyword.value"
 :placeholder="`关键词 ${index + 1}`"
 clearable
 />
 <n-tooltip trigger="hover">
 <template #trigger>
 <n-button
 quaternary
 circle
 type="error"
 @click="removeKeyword(keyword.id)"
 >
 <template #icon>
 <n-icon><TrashOutline /></n-icon>
 </template>
 </n-button>
 </template>
 删除关键词
 </n-tooltip>
 </div>
 </div>
 </section>
 </div>

 <aside class="result-area">
 <section class="panel result-panel">
 <div class="panel-heading panel-heading-row">
 <div>
 <h2>生成结果</h2>
 <n-text depth="3">修改条件后自动更新</n-text>
 </div>
 <div class="result-actions">
 <n-button
 secondary
 :disabled="!hasExpression"
 :loading="openingCls"
 @click="handleOpenCls"
 >
 <template #icon>
 <n-icon><OpenOutline /></n-icon>
 </template>
 直接跳转
 </n-button>
 <n-button
 type="primary"
 :disabled="!hasExpression"
 @click="handleCopy"
 >
 <template #icon>
 <n-icon>
 <ClipboardOutline v-if="copied" />
 <CopyOutline v-else />
 </n-icon>
 </template>
 {{ copied ? '已复制' : '复制' }}
 </n-button>
 </div>
 </div>

 <div v-if="hasExpression" class="expression-box">
 {{ expression }}
 </div>

 <n-empty
 v-else
 class="empty-result"
 description="请选择查询条件"
 />

 <div class="condition-list" v-if="hasExpression">
 <div
 v-for="(part, index) in expressionParts"
 :key="`${index}-${part}`"
 class="condition-item"
 >
 {{ part }}
 </div>
 </div>
 </section>
 </aside>
 </section>
 </main>
</template>

<style scoped>
.log-expression-page {
 width: 100%;
 height: 100%;
 min-height: 0;
 overflow: auto;
 color: var(--n-text-color-1, #333639);
 background:
 linear-gradient(180deg, rgba(32, 128, 240, 0.08), transparent 280px),
 var(--n-body-color, #f5f7fa);
}

.page-header {
 height: 64px;
 padding: 0 28px;
 display: flex;
 align-items: center;
 justify-content: space-between;
 border-bottom: 1px solid var(--n-border-color, #e0e0e6);
 background-color: color-mix(in srgb, var(--n-card-color, #ffffff) 88%, transparent);
}

.header-left {
 min-width: 0;
 display: flex;
 align-items: center;
 gap: 12px;
}

.icon-link {
 width: 34px;
 height: 34px;
 flex: none;
 display: grid;
 place-items: center;
 border-radius: 6px;
 color: var(--n-text-color-2, #666666);
 transition: background-color 0.15s ease, color 0.15s ease;
}

.icon-link:hover {
 color: var(--n-primary-color, #18a058);
 background-color: var(--n-action-color, rgba(0, 0, 0, 0.04));
}

.title-mark {
 width: 40px;
 height: 40px;
 flex: none;
 display: grid;
 place-items: center;
 border-radius: 8px;
 color: #ffffff;
 background-color: #2080f0;
}

.title-copy {
 min-width: 0;
}

.title-copy h1 {
 margin-bottom: 2px;
 font-size: 18px;
 line-height: 1.2;
}

.page-shell {
 width: min(1180px, calc(100vw - 48px));
 margin: 0 auto;
 padding: 32px 0;
 display: grid;
 gap: 18px;
}

.form-area {
 min-width: 0;
 display: grid;
 gap: 18px;
}

.result-area {
 min-width: 0;
}

.panel {
 min-width: 0;
 padding: 20px;
 border: 1px solid var(--n-border-color, #e0e0e6);
 border-radius: 8px;
 background-color: var(--n-card-color, #ffffff);
 box-shadow: var(--shadow-sm, 0 1px 2px rgba(0, 0, 0, 0.05));
}

.panel-heading {
 margin-bottom: 16px;
}

.panel-heading h2 {
 font-size: 18px;
 line-height: 1.25;
}

.panel-heading-row {
 display: flex;
 align-items: center;
 justify-content: space-between;
 gap: 12px;
}

.selector-grid {
 display: grid;
 gap: 14px;
}

.field-block {
 min-width: 0;
 display: grid;
 gap: 7px;
}

.field-block > span {
 font-size: 13px;
 font-weight: 600;
 color: var(--n-text-color-2, #666666);
}

.selection-summary {
 margin-top: 18px;
 display: grid;
 grid-template-columns: repeat(4, minmax(0, 1fr));
 gap: 10px;
}

.selection-summary > div {
 min-width: 0;
 padding: 11px 12px;
 border-radius: 6px;
 background-color: var(--n-action-color, rgba(0, 0, 0, 0.04));
}

.selection-summary :deep(.n-text) {
 display: block;
 margin-bottom: 4px;
 font-size: 12px;
}

.selection-summary strong {
 display: block;
 min-width: 0;
 overflow: hidden;
 text-overflow: ellipsis;
 white-space: nowrap;
 font-size: 13px;
 color: var(--n-text-color-1, #333639);
}

.keyword-list {
 display: grid;
 gap: 10px;
}

.keyword-row {
 display: grid;
 grid-template-columns: minmax(0, 1fr) 34px;
 gap: 8px;
 align-items: center;
}

.result-actions {
 display: flex;
 flex-wrap: wrap;
 justify-content: flex-end;
 gap: 8px;
}

.result-panel {
 display: grid;
 gap: 14px;
}

.result-panel .panel-heading {
 margin-bottom: 0;
}

.expression-box {
 min-height: 164px;
 padding: 14px;
 border: 1px solid var(--n-border-color, #e0e0e6);
 border-radius: 8px;
 color: var(--n-text-color-1, #333639);
 background-color: var(--n-action-color, rgba(0, 0, 0, 0.04));
 font-family: var(--font-family-mono, Consolas, monospace);
 font-size: 13px;
 line-height: 1.7;
 overflow-wrap: anywhere;
 white-space: pre-wrap;
 user-select: text;
}

.empty-result {
 min-height: 164px;
 display: grid;
 place-items: center;
 border: 1px dashed var(--n-border-color, #e0e0e6);
 border-radius: 8px;
}

.condition-list {
 display: grid;
 gap: 8px;
}

.condition-item {
 padding: 9px 10px;
 border-radius: 6px;
 color: var(--n-text-color-2, #666666);
 background-color: var(--n-action-color, rgba(0, 0, 0, 0.04));
 font-family: var(--font-family-mono, Consolas, monospace);
 font-size: 12px;
 line-height: 1.5;
 overflow-wrap: anywhere;
}

@media (max-width: 720px) {
 .page-header {
 height: 56px;
 padding: 0 16px;
 }

 .title-mark {
 width: 34px;
 height: 34px;
 }

 .title-copy h1 {
 font-size: 16px;
 }

 .title-copy :deep(.n-text) {
 display: none;
 }

 .page-shell {
 width: calc(100vw - 28px);
 padding: 20px 0;
 }

 .panel {
 padding: 16px;
 }

 .panel-heading-row {
 align-items: flex-start;
 }

 .selection-summary {
 grid-template-columns: repeat(2, minmax(0, 1fr));
 }
}

@media (max-width: 420px) {
 .selection-summary {
 grid-template-columns: 1fr;
 }

 .panel-heading-row {
 flex-direction: column;
 }

 .panel-heading-row :deep(.n-button) {
 width: 100%;
 }

 .result-actions {
 width: 100%;
 }
}
</style>
