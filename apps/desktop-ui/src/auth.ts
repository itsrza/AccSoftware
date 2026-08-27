import {api} from './api'

export const logout=()=>api<void>('logout')
