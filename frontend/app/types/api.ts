export interface AuthUser {
  id: string
  name: string
  email: string
  role: 'owner' | 'admin' | 'instructor' | 'learner' | string
}

export interface AuthCompany {
  id: string
  name: string
  slug: string
  status?: string
}

export interface AuthResponse {
  accessToken: string
  tokenType: string
  expiresIn: number
  user: AuthUser
  company: AuthCompany
}

export interface LoginInput {
  companySlug: string
  email: string
  password: string
}

export interface RegisterInput extends LoginInput {
  companyName: string
  name: string
}

export interface Course {
  id: string
  title: string
  slug: string
  description?: string | null
  priceInPaise: number
  currency: string
  status: 'draft' | 'published' | string
  createdAtMs: number
}

export interface CreateCourseInput {
  title: string
  slug: string
  description?: string
  priceInPaise: number
}