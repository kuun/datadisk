import { ElMessage } from 'element-plus'
import i18n from '../i18n';

export default {
    // 国际化函数
    t(key) {
        return i18n.global.t(key);
    },

    t(key, params) {
        return i18n.global.t(key, params);
    },

    formatFileSize(fileSize) {
        if (fileSize < 1024) {
            return fileSize + 'B';
        } else if (fileSize < (1024 * 1024)) {
            let temp = fileSize / 1024;
            temp = temp.toFixed(2);
            return temp + 'KB';
        } else if (fileSize < (1024 * 1024 * 1024)) {
            let temp = fileSize / (1024 * 1024);
            temp = temp.toFixed(2);
            return temp + 'MB';
        } else if (fileSize < (1024 * 1024 * 1024 * 1024)) {
            let temp = fileSize / (1024 * 1024 * 1024);
            temp = temp.toFixed(2);
            return temp + 'GB';
        } else {
            let temp = fileSize / (1024 * 1024 * 1024 * 1024);
            temp = temp.toFixed(2);
            return temp + 'PB';
        }
    },

    unitToByte(size, unit) {
        switch (unit) {
            case 'KB':
                return size * 1024;
            case 'MB':
                return size * 1024 * 1024;
            case 'GB':
                return size * 1024 * 1024 * 1024;
            case 'TB':
                return size * 1024 * 1024 * 1024 * 1024;
            case 'PB':
                return size * 1024 * 1024 * 1024 * 1024 * 1024;
            default:
                return size;
        }
    },

    byteToUnit(size) {
        if (size < 1024) {
            return size + ' B';
        } else if (size < (1024 * 1024)) {
            let temp = size / 1024;
            temp = temp.toFixed(0);
            return temp + ' KB';
        } else if (size < (1024 * 1024 * 1024)) {
            let temp = size / (1024 * 1024);
            temp = temp.toFixed(0);
            return temp + ' MB';
        } else if (size < (1024 * 1024 * 1024 * 1024)) {
            let temp = size / (1024 * 1024 * 1024);
            temp = temp.toFixed(0);
            return temp + ' GB';
        } else {
            let temp = size / (1024 * 1024 * 1024 * 1024);
            temp = temp.toFixed(0);
            return temp + ' TB';
        }
    },

    alertError(msg) {
        ElMessage({
            message: msg,
            grouping: true,
            type: 'error',
        })
    },

    alertSuccess(msg) {
        ElMessage({
            message: msg,
            grouping: true,
            type: 'success',
        })
    }
}