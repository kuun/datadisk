import {defineStore} from "pinia";
import {computed, ref} from "vue";

export const useGroupsStore = defineStore('groups', () => {
  const groups = ref([])

  function setGroups(data) {
    groups.value = data
  }

  const selectedGroups = computed(() => {
    return groups.value
  })

  return { groups, setGroups, selectedGroups }
})