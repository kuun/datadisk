import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export const useLoginStore = defineStore('login', () => {
    const loginUser = ref('')
    const userAvatar = ref('')  // 添加头像状态

    function setLoginUser(user) {
        loginUser.value = user
        // 设置用户时同时更新头像URL
        userAvatar.value = `/api/user/avatar/${user}`
    }

    function updateAvatar(timestamp) {
        // 通过添加时间戳强制刷新头像
        userAvatar.value = `/api/user/avatar/${loginUser.value}?t=${timestamp}`
    }

    function clearLoginUser() {
        loginUser.value = ''
        userAvatar.value = ''
    }

    const getLoginUser = computed(() => loginUser.value)
    const getAvatar = computed(() => userAvatar.value)

    return { 
        loginUser, 
        userAvatar,
        setLoginUser, 
        updateAvatar,
        clearLoginUser, 
        getLoginUser,
        getAvatar 
    }
}, {
    persist: {
        enabled: true,
        strategies: [
            {
                key: 'loginUser',
                storage: localStorage,
            },
        ],
    },
})
