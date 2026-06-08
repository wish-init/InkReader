import {
  call,
  type Book,
  type BookListRequest,
  type BookListResponse,
  type BookThumbnail,
  type Chapter,
  type FavoriteCollection,
  type UpdateBookMetadataRequest,
} from './tauri'

export function listBooks(request: BookListRequest = {}): Promise<BookListResponse> {
  return call('list_books', { request: normalizeBookListRequest(request) })
}

export function ensureBookThumbnails(bookIds: string[]): Promise<BookThumbnail[]> {
  return call('ensure_book_thumbnails', { bookIds })
}

export function listFavoriteBooks(request: BookListRequest = {}): Promise<BookListResponse> {
  return call('list_favorite_books', { request: normalizeBookListRequest(request) })
}

export function listBookTags(repositoryId?: string): Promise<string[]> {
  return call('list_book_tags', { repositoryId: repositoryId ?? null })
}

function normalizeBookListRequest(request: BookListRequest): BookListRequest {
  return {
    repositoryId: request.repositoryId ?? null,
    collectionId: request.collectionId ?? null,
    query: request.query ?? null,
    tag: request.tag ?? null,
    tags: request.tags ?? null,
    readingStatus: request.readingStatus ?? 'all',
    favoriteStatus: request.favoriteStatus ?? 'all',
    sortKey: request.sortKey ?? 'createdAt',
    sortDirection: request.sortDirection ?? 'desc',
    limit: request.limit ?? 80,
    offset: request.offset ?? 0,
  }
}

export function listFavoriteCollections(): Promise<FavoriteCollection[]> {
  return call('list_favorite_collections')
}

export function createFavoriteCollection(name: string): Promise<FavoriteCollection> {
  return call('create_favorite_collection', { name })
}

export function renameFavoriteCollection(collectionId: string, name: string): Promise<FavoriteCollection> {
  return call('rename_favorite_collection', { collectionId, name })
}

export function deleteFavoriteCollection(collectionId: string): Promise<void> {
  return call('delete_favorite_collection', { collectionId })
}

export function addBookToFavoriteCollection(bookPath: string, collectionId: string): Promise<void> {
  return call('add_book_to_favorite_collection', { bookPath, collectionId })
}

export function addBooksToFavoriteCollection(bookPaths: string[], collectionId: string): Promise<void> {
  return call('add_books_to_favorite_collection', { bookPaths, collectionId })
}

export function removeBookFromFavoriteCollection(bookPath: string, collectionId: string): Promise<void> {
  return call('remove_book_from_favorite_collection', { bookPath, collectionId })
}

export function removeBooksFromFavoriteCollection(bookPaths: string[], collectionId: string): Promise<void> {
  return call('remove_books_from_favorite_collection', { bookPaths, collectionId })
}

export function moveBooksBetweenFavoriteCollections(
  bookPaths: string[],
  sourceCollectionId: string,
  targetCollectionId: string,
): Promise<void> {
  return call('move_books_between_favorite_collections', { bookPaths, sourceCollectionId, targetCollectionId })
}

export function removeBooksFromAllFavoriteCollections(bookPaths: string[]): Promise<void> {
  return call('remove_books_from_all_favorite_collections', { bookPaths })
}

export function listBookFavoriteCollections(bookPath: string): Promise<FavoriteCollection[]> {
  return call('list_book_favorite_collections', { bookPath })
}

export function setBookFavorite(bookPath: string, favorite: boolean): Promise<void> {
  return call('set_book_favorite', { bookPath, favorite })
}

export function renameBookTitle(bookPath: string, title: string): Promise<Book> {
  return call('rename_book_title', { bookPath, title })
}

export function resetBookTitle(bookPath: string): Promise<Book> {
  return call('reset_book_title', { bookPath })
}

export function updateBookMetadata(request: UpdateBookMetadataRequest): Promise<Book> {
  return call('update_book_metadata', { request })
}

export function getBook(bookId: string): Promise<Book> {
  return call('get_book', { bookId })
}

export function listBookChapters(bookId: string): Promise<Chapter[]> {
  return call('list_book_chapters', { bookId })
}
