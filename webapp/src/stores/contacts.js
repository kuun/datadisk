import {defineStore} from 'pinia'
import {ref, computed} from 'vue'

export const useContactsStore = defineStore('contacts', () => {
  const contacts = ref([])

    function setContacts(data) {
        contacts.value = data
    }

    const selectedContacts = computed(() => {
        return contacts.value
    })

    return { contacts, setContacts, selectedContacts }
})