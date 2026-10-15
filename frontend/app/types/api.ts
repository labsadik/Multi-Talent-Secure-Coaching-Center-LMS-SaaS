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
  user: AuthUser
  company: AuthCompany
  expiresIn: number
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
export interface LearnerRegisterInput extends LoginInput {
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
export interface ContentItem {
  id: string
  title: string
  slug: string
  description?: string | null
  status: string
  sortOrder: number
  createdAtMs: number
  courseId?: string | null
  subjectId?: string | null
  topicId?: string | null
  lessonType?: 'text' | 'video' | 'file' | string | null
  content?: string | null
  mediaAssetId?: string | null
  durationSeconds?: number | null
  isPreview?: boolean | null
}
export interface CompanyUser {
  id: string
  name: string
  email: string
  role: string
  status: string
  createdAtMs?: number
}
export interface LearnerCourse {
  courseId: string
  title: string
  slug: string
  description?: string
  status?: string
  progressPercent?: number
  completedLessons?: number
  totalLessons?: number
  enrolledAt?: number
}
export interface CourseProgress {
  courseId: string
  courseTitle?: string
  totalLessons?: number
  completedLessons?: number
  progressPercent?: number
  lessons?: Array<{ lessonId: string; title?: string; status?: string; positionSeconds?: number }>
}
export interface PlaybackInfo {
  lessonId: string
  assetId?: string
  lessonType: string
  provider?: string
  playbackUrl?: string
  downloadUrl?: string
  contentType?: string
  content?: string
  expiresAt?: number
}